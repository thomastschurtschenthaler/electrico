use std::{any::Any, collections::HashMap, fs::File, hash::{DefaultHasher, Hash, Hasher}, path::PathBuf, sync::mpsc::{self, Receiver, Sender}};
use muda::MenuId;
use notify::{Event, RecommendedWatcher};
use substring::Substring;
use log::{debug, error, trace};
use include_dir::{include_dir, Dir};
use tao::{dpi::PhysicalSize, event_loop::{EventLoop, EventLoopProxy}, window::{Window, WindowBuilder}};
use tokio::runtime::Runtime;
use uuid::Uuid;
use wry::{http::Request, RequestAsyncResponder, WebView, WebViewBuilder};
use serde_json::Error;
use crate::{common::{append_js_scripts, build_file_map, escape, get_message_data, handle_file_request, is_module_request, respond_404, DataQueue}, types::{BackendCommand, ChildProcess, NETConnection, NETServer}};
use crate::types::{Package, ElectricoEvents, Command};

pub struct Backend {
    window:Window,
    package:Package,
    src_dir:PathBuf,
    webview:WebView,
    webviews:HashMap<String, WebView>,
    command_sender:Sender<BackendCommand>,
    command_receiver:Receiver<BackendCommand>,
    child_process:HashMap<String, tokio::sync::mpsc::Sender<ChildProcess>>,
    fs_watcher:HashMap<String, RecommendedWatcher>,
    fs_files:HashMap<i64, File>,
    net_server:HashMap<String, tokio::sync::mpsc::Sender<NETServer>>,
    net_connections:HashMap<String, tokio::sync::mpsc::Sender<NETConnection>>,
    data_queue:DataQueue,
    addon_state: HashMap<String, Box<dyn Any>>,
    tokio_runtime:Runtime
}

fn create_web_view (
        window:&Window, 
        proxy:EventLoopProxy<ElectricoEvents>,
        backend_js_files: HashMap<String, Vec<u8>>,
        src_dir:&PathBuf,
        package:&Package,
        init_script:String) -> WebView {
    let mut is_windows="false";
    #[cfg(target_os = "windows")] {
        is_windows = "true";
    }
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let builder = WebViewBuilder::new(window);
    
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let builder = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        WebViewBuilder::new_gtk(vbox)
    };
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(20).enable_io().enable_time().build().unwrap();
    let src_dir_fil = src_dir.clone();
    let fil_handler = move |request: Request<Vec<u8>>, responder:RequestAsyncResponder| {
        let rpath = request.uri().path().to_string();
        trace!("backend fil: request {}", rpath);
        let fpath = rpath.substring(1, rpath.len()).to_string();
        let file:PathBuf;
        if fpath.starts_with("/") {
            file = PathBuf::from(fpath.clone());
        } else {
            file = src_dir_fil.join(fpath.clone());
        } 
        trace!("trying load file {}", file.clone().as_mut_os_str().to_str().unwrap());
        handle_file_request(&tokio_runtime, is_module_request(request.uri().host()), fpath, file, &backend_js_files, crate::types::Responder::CustomProtocol { responder });
    };
    let cmd_handler = move |request: Request<Vec<u8>>, responder:RequestAsyncResponder| {
        let path = request.uri().path().to_string();
        trace!("backend cmd request {} {}", path, request.body().len());
        let message_data:Option<(String, Option<Vec<u8>>)> = get_message_data(&request);
        
        if let Some(message_data) = message_data {
            let commandr:Result<Command, Error> = serde_json::from_str(message_data.0.as_str());
            match commandr {
                Ok (command) => {
                    let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command, responder:crate::types::Responder::CustomProtocol { responder: responder }, data_blob:message_data.1});
                }
                Err(e) => {
                    error!("json serialize error {}, {}", e.to_string(), message_data.0);
                    respond_404(crate::types::Responder::CustomProtocol { responder });
                    return;
                }
            }
        } else {
            respond_404(crate::types::Responder::CustomProtocol { responder });
        }
    };
    
    let mut hasher = DefaultHasher::new();
    format!("{}/{}", src_dir.as_os_str().to_str().unwrap(), package.main.to_string().as_str()).hash(&mut hasher);
    let hash = format!("{}", hasher.finish());
    let main = package.main.clone();
    let pid = std::process::id();
    let webview = builder
        .with_url(format!("e{hash}://file/{main}-{pid}"))
        .with_asynchronous_custom_protocol(format!("e{}", hash).into(), fil_handler)
        .with_asynchronous_custom_protocol("cmd".into(), cmd_handler)
        .with_devtools(true)
        .with_incognito(false)
        .with_initialization_script(("window.__is_windows=".to_string()+is_windows+";"+init_script.as_str()).as_str())
        .build().unwrap();

    #[cfg(debug_assertions)]
    webview.open_devtools();
    webview
}

fn backend_resources (package:&Package) -> (HashMap<String, Vec<u8>>, String) {
    let mut backendjs:String = String::new();
    const JS_DIR_SHARED: Dir = include_dir!("src/js/shared");
    backendjs = append_js_scripts(backendjs, JS_DIR_SHARED, Some(".js"));
    const JS_DIR_BACKEND: Dir = include_dir!("src/js/backend");
    backendjs = append_js_scripts(backendjs, JS_DIR_BACKEND, Some("electrico.js"));
    let mut backend_js_files = build_file_map(&JS_DIR_BACKEND);
    backend_js_files.remove("package.json");
    if let Some(fork) = &package.fork {
        backendjs = backendjs+format!("\nwindow.__electrico.init_fork('{}', '{}', '{}');", fork.0, fork.1, escape(&fork.2)).as_str();
    }
    (backend_js_files, backendjs)
}

impl Backend {
    pub fn new(src_dir:PathBuf, package:Package, event_loop:&EventLoop<ElectricoEvents>, proxy:EventLoopProxy<ElectricoEvents>) -> Backend {
        let (command_sender, command_receiver): (Sender<BackendCommand>, Receiver<BackendCommand>) = mpsc::channel();

        let (backend_js_files, backendjs) = backend_resources(&package);
        
        let init_script = backendjs+"\nwindow.__electrico.loadMain('"+package.main.to_string().as_str()+"');";
        
        let mut window_builder = WindowBuilder::new()
            .with_title("Electrico Node backend");

        #[cfg(target_os = "macos")] {
            #[cfg(debug_assertions)] {
                window_builder = window_builder.with_inner_size(PhysicalSize::new(1,1));
            }
            #[cfg(not(debug_assertions))] {
                window_builder = window_builder.with_visible(false);
            }
        }
        #[cfg(not(target_os = "macos"))] {
            window_builder = window_builder.with_visible(false);
        }

        let window = window_builder
            .build(event_loop)
            .unwrap();
        
        let webview = create_web_view(&window, proxy, backend_js_files, &src_dir, &package, init_script);
        
        Backend {
            window:window,
            webview:webview,
            webviews:HashMap::new(),
            package:package,
            src_dir:src_dir,
            command_sender,
            command_receiver,
            child_process: HashMap::new(),
            fs_watcher: HashMap::new(),
            fs_files: HashMap::new(),
            net_server: HashMap::new(),
            net_connections: HashMap::new(),
            data_queue: DataQueue::new(),
            addon_state: HashMap::new(),
            tokio_runtime:tokio::runtime::Builder::new_multi_thread().worker_threads(20).enable_io().enable_time().build().unwrap()
        }
    }
    pub fn addon_state_insert<T: 'static>(&mut self, cid:&String, c:T) {
        self.addon_state.insert(cid.clone(), Box::new(c));
    }
    pub fn addon_state_remove(&mut self, cid:&String) {
        if let Some(_d) = self.addon_state.get(cid) {
            self.addon_state.remove(cid);
        }
    }
    pub fn addon_state_get_mut<T: 'static>(&mut self, cid:&String) -> Option<&mut T> {
        if let Some(b) = self.addon_state.get_mut(cid) {
            return b.as_mut().downcast_mut::<T>();
        }
        return None;
    }
    pub fn command_callback(&mut self, command:String, message:String) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.callback['{}']('{}')", command, message).as_str());
    }
    pub fn call_ipc_channel(&mut self, browser_window_id:String, request_id:String, params:String, data_blob:Option<Vec<u8>>) {
         let request_id2 = request_id.clone();
         trace!("call_ipc_channel {} {}", &request_id2, &params);
         if let Some(data) = data_blob {
            if self.data_queue.add(&request_id, data) {
                return;
            }
        }
        let retry_sender = self.command_sender.clone();
         _ = self.webview.evaluate_script_with_callback(
            format!("window.__electrico.callIPCChannel('{}', '{}', '{}');", browser_window_id, request_id, escape(&params)).as_str()
            , move |r| {
                if r.len()==0 {
                    trace!("call_ipc_channel not OK - resending");
                    let _ = retry_sender.send(BackendCommand::IPCCall { browser_window_id:browser_window_id.clone(), request_id:request_id.clone(), params:params.clone() });
                }
        });
    }
    pub fn window_close(&mut self, id:&Option<String>) {
        if let Some(id) = id {
            let _ = self.webview.evaluate_script(format!("window.__electrico.callAppOn('window-close', '{}');", id).as_str());
        } else {
            let _ = self.webview.evaluate_script(format!("window.__electrico.callAppOn('window-close');").as_str());
        }
    }
    pub fn window_focus(&mut self, id:&String, focus:bool) {
        let ev = if focus {"browser-window-focus"} else {"browser-window-blur"};
        let _ = self.webview.evaluate_script(format!("window.__electrico.callAppOn('{ev}', '{id}');").as_str());
    }
    pub fn window_all_closed(&mut self) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.callAppOn('window-all-closed');").as_str());
    }
    pub fn menu_selected(&mut self, id:MenuId) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.menuSelected('{}');", id.as_ref()).as_str());
    }
    pub fn dom_content_loaded(&mut self, id:&String) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.domContentLoaded('{}');", id).as_str());
    }
    pub fn child_process_callback(&mut self, pid:String, stream:String, end:bool, data:Option<Vec<u8>>) {
        if stream=="stdin" {
            if let Some(sender) = self.child_process.get(&pid) {
              trace!("ChildProcessData stdin {} {:?}", pid, data);
            let sender = sender.clone();
            self.tokio_runtime.spawn(async move {
                let _ = sender.send(ChildProcess::StdinWrite {data, end}).await;
            });
            }
        } else {
            trace!("child_process_callback {} {}", stream, pid);
            if let Some(data) = data {
                let data_key = pid.clone()+stream.as_str();
                if self.data_queue.add(&data_key, data) {
                    return;
                }
            }
            let retry_sender = self.command_sender.clone();
            let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{window.__electrico.child_process.callback.on_{}('{}');}});", stream, pid), move |r| {
                if r.len()==0 {
                    trace!("child_process_callback not OK - resending");
                    let _ = retry_sender.send(BackendCommand::ChildProcessCallback { pid:pid.clone(), end, stream:stream.clone(), data:None });
                }
            });
        }
    }
    pub fn child_process_exit(&mut self, pid:String, exit_code:Option<i32>) {
        let call_script:String;
        if let Some(exit_code) = exit_code {
            call_script=format!("window.__electrico.child_process.callback.on_close('{}', {});", pid, exit_code.to_string());
        } else {
            call_script=format!("window.__electrico.child_process.callback.on_close('{}');", pid);
        }
        let retry_sender = self.command_sender.clone();
        if self.data_queue.size(&(pid.clone()+"stdout")) == 0 && self.data_queue.size(&(pid.clone()+"stderr")) == 0 {
            let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
                if r.len()==0 {
                    trace!("child_process_exit not OK - resending");
                    let _ = retry_sender.send(BackendCommand::ChildProcessExit { pid: pid.clone(), exit_code: exit_code.clone() });
                }
            });
        } else {
            trace!("still stdout/stderr data on queue - call exit later {}", pid);
            let _ = retry_sender.send(BackendCommand::ChildProcessExit { pid: pid.clone(), exit_code: exit_code.clone() });
        }
    }
    pub fn fs_watch_callback(&mut self, wid:String, event:Event) {
        let call_script = format!("window.__electrico.fs_watcher.on_event('{}', '{:?}', '{}')",
            wid, 
            event.kind,
            escape(&event.paths.iter().map(|x| x.as_os_str().to_str().unwrap()).collect::<Vec<_>>().join(";")));
        let retry_sender = self.command_sender.clone();
        let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
            if r.len()==0 {
                trace!("fs_watch_callback not OK - resending");
                let _ = retry_sender.send(BackendCommand::FSWatchEvent { wid:wid.clone(), event:event.clone() });
            }
        });
    }
    pub fn command_sender(&mut self) -> Sender<BackendCommand> {
        self.command_sender.clone()
    }
    pub fn child_process_start(&mut self, pid:&String, sender:tokio::sync::mpsc::Sender<ChildProcess>) {
        trace!("child_process_start {}", pid);
        self.child_process.insert(pid.clone(), sender);
    }
    pub fn child_process_disconnect(&mut self, pid:String) {
        trace!("child_process_disconnect {}", pid);
        if let Some(sender) = self.child_process.get(&pid) {
            let sender = sender.clone();
            self.tokio_runtime.spawn(async move {
                let _ = sender.send(ChildProcess::Disconnect).await;
            });
        }
    }
    pub fn child_process_kill(&mut self, pid:String) {
        trace!("child_process_kill {}", pid);
        if let Some(sender) = self.child_process.get(&pid) {
            let sender = sender.clone();
            self.tokio_runtime.spawn(async move {
                let _ = sender.send(ChildProcess::Kill).await;
            });
        }
    }
    pub fn fs_open(&mut self, fd:i64, file:File) {
        trace!("fs_open {}", fd);
        self.fs_files.insert(fd, file);
    }
    pub fn fs_close(&mut self, fd:i64) {
        trace!("fs_close {}", fd);
        if let Some(_file) = self.fs_files.get(&fd) {
            self.fs_files.remove(&fd);
        }
    }
    pub fn fs_get(&mut self, fd:i64) -> Option<&File>{
        trace!("fs_get {}", fd);
        return self.fs_files.get(&fd);
    }
    pub fn watch_start(&mut self, wid:String, watcher:RecommendedWatcher) {
        trace!("watch_start {}", wid);
        self.fs_watcher.insert(wid, watcher);
    }
    pub fn watch_stop(&mut self, wid:String) {
        trace!("watch_stop {}", wid);
        if let Some(_watcher) = self.fs_watcher.get(&wid) {
            self.fs_watcher.remove(&wid);
        }
    }
    pub fn net_server_conn_start(&mut self, hook:String, id:String, sender:tokio::sync::mpsc::Sender<NETConnection>) {
        let call_script=format!("window.__electrico.net_server.callback.on_start('{}', '{}');", hook, id);
        self.net_connections.insert(id.clone(), sender.clone());
        let retry_sender = self.command_sender.clone();
        let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
            if r.len()==0 {
                trace!("net_server_conn_start not OK - resending");
                let _ = retry_sender.send(BackendCommand::NETServerConnStart { hook:hook.clone(), id:id.clone(), sender:sender.clone()});
            }
        });
    }
    pub fn net_server_close(&mut self, id:String) {
        if let Some(sender) = self.net_server.get(&id) {
            let _ = sender.send(NETServer::Close);
            self.net_server.remove(&id);
        }
    }
    pub fn net_connection_close(&mut self, id:String) {
        if let Some(sender) = self.net_connections.get(&id) {
            let _ = sender.send(NETConnection::EndConnection);
        }
    }
    pub fn net_client_conn_start(&mut self, id:String, sender:tokio::sync::mpsc::Sender<NETConnection>) {
        self.net_connections.insert(id.clone(), sender.clone());
    }
    pub fn net_connection_data(&mut self, id:String, data:Option<Vec<u8>>) {
        if let Some(data) = data {
            if  self.data_queue.add(&id, data) {
                return;
            }
        }
        let call_script=format!("window.__electrico.net_server.callback.on_data('{}');", id);
        let retry_sender = self.command_sender.clone();
        let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
            if r.len()==0 {
                trace!("net_connection_data not OK - resending");
                let _ = retry_sender.send(BackendCommand::NETConnectionData { id:id.clone(), data:None });
            }
        });
    }
    pub fn net_connection_end(&mut self, id:String) {
        if let Some(sender) = self.net_connections.get(&id) {
            let _ = sender.send(NETConnection::Disconnect);
            self.net_connections.remove(&id);
        }
        let call_script=format!("window.__electrico.net_server.callback.on_end('{}');", id);
        let retry_sender = self.command_sender.clone();
        
        if self.data_queue.size(&id) == 0 {
            let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
                if r.len()==0 {
                    trace!("net_connection_end not OK - resending");
                    let _ = retry_sender.send(BackendCommand::NETConnectionEnd { id:id.clone()});
                }
            });
        } else {
            debug!("still connection data on queue - call end later");
            let _ = retry_sender.send(BackendCommand::NETConnectionEnd { id:id.clone()});
        }
    }
    pub fn net_write_connection(&mut self, id:String, data:Vec<u8>) {
        if let Some(sender) = self.net_connections.get(&id) {
            let sender = sender.clone();
            self.tokio_runtime.spawn(async move {
                let _ = sender.send(NETConnection::Write { data }).await;
            });
        } else {
            error!("net_write_connection no sender for id {}", id);
        }
    }
    pub fn net_set_timeout(&mut self, id:String, timeout:u128) {
        if let Some(sender) = self.net_connections.get(&id) {
            let sender = sender.clone();
            self.tokio_runtime.spawn(async move {
                let _ = sender.send(NETConnection::SetTimeout { timeout:Some(timeout) }).await;
            });
        } else {
            error!("net_write_connection no sender for id {}", id);
        }
    }
    pub fn get_data_blob(&mut self, id:String) -> Option<Vec<u8>> {
        let data:Option<Vec<u8>>;
        if let Some(d) = self.data_queue.take(&id) {
            data = Some(d.to_vec());
        } else {
            data = None;
        };
        return data;
    }
    pub fn execute_sync(&mut self, proxy:EventLoopProxy<ElectricoEvents>, script:String, sender:Sender<(bool, Vec<u8>)>) {
        let (backend_js_files, backendjs) = backend_resources(&self.package);
        let init_script = format!("{}\nwindow.__electrico.loadMain();", backendjs);
        let webview = create_web_view(&self.window, proxy, backend_js_files, &self.src_dir, &self.package, init_script);
        let uuid = Uuid::new_v4().to_string();
        let _ = webview.evaluate_script(format!("{script}.then(r=>{{$e_node.syncExecuteSyncResponse({{'uuid':'{uuid}', 'data':r+''}});}}).catch(e=>{{$e_node.syncExecuteSyncResponse({{'uuid':'{uuid}', 'error':e+''}});}});").as_str());
        self.addon_state_insert(&uuid, sender);
        self.webviews.insert(uuid, webview);
    }
    pub fn execute_sync_response(&mut self, uuid:String, data:Option<String>, error:Option<String>) {
        self.webviews.remove(&uuid);
        let sender:Option<&mut Sender<(bool, Vec<u8>)>> = self.addon_state_get_mut(&uuid);
        if let Some(sender) = sender {
            if let Some(data) = data {
                let _ = sender.send((true, data.as_bytes().to_vec()));
            } else if let Some(error) = error {
                let _ = sender.send((false, error.as_bytes().to_vec()));
            } else {
                let _ = sender.send((false, "no vaild response".as_bytes().to_vec()));
            }
        }
        self.addon_state_remove(&uuid);
    }
    pub fn process_commands(&mut self) {
        if let Ok(command) = self.command_receiver.try_recv() {
            match command {
                BackendCommand::IPCCall { browser_window_id, request_id, params } => {
                    self.call_ipc_channel(browser_window_id, request_id, params, None);
                },
                BackendCommand::ChildProcessCallback { pid, stream, end, data } => {
                    trace!("ChildProcessCallback");
                    self.child_process_callback(pid, stream, end, data);
                },
                BackendCommand::ChildProcessExit { pid, exit_code } => {
                    trace!("ChildProcessExit");
                    if self.child_process.contains_key(&pid) {
                        self.child_process.remove(&pid);
                    }
                    self.child_process_exit(pid, exit_code);
                }
                BackendCommand::FSWatchEvent { wid, event } => {
                    trace!("FSWatchEvent");
                    self.fs_watch_callback(wid, event);
                },
                BackendCommand::NETServerStart { id, sender } => {
                    trace!("NETServerStart");
                    self.net_server.insert(id, sender);
                }
                BackendCommand::NETServerConnStart { hook, id, sender } => {
                    trace!("NETServerConnStart {}", hook);
                    self.net_server_conn_start(hook, id, sender);
                },
                BackendCommand::NETClientConnStart { id, sender } => {
                    trace!("NETClientConnStart {}", id);
                    self.net_client_conn_start(id, sender);
                },
                BackendCommand::NETConnectionData {id,  data } => {
                    trace!("NETConnectionData {}", id);
                    self.net_connection_data(id, data);
                },
                BackendCommand::NETConnectionEnd { id } => {
                    trace!("NETServerConnEnd {}", id);
                    self.net_connection_end(id);
                },
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.fs_watcher.clear();
        self.fs_files.clear();
        self.net_server.clear();
        self.net_connections.clear();
        self.addon_state.clear();
    }
}