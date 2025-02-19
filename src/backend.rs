use std::{any::Any, collections::HashMap, convert::Infallible, fs::File, hash::{DefaultHasher, Hash, Hasher}, net::TcpListener, path::PathBuf, sync::mpsc::{self, Receiver, Sender}, thread, time::Duration};
use bytes::{Bytes, BytesMut, BufMut};
use fastwebsockets::{upgrade::{self}, Frame, OpCode, Payload};
use http_body_util::{BodyExt, Full};
use hyper::{header::ACCESS_CONTROL_ALLOW_ORIGIN, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use muda::MenuId;
use notify::{Event, RecommendedWatcher};
use substring::Substring;
use log::{debug, error, trace};
use include_dir::{include_dir, Dir};
use tao::{dpi::PhysicalSize, event_loop::{EventLoop, EventLoopProxy}, window::{Window, WindowBuilder}};
use tokio::time::timeout;
use uuid::Uuid;
use wry::{WebView, WebViewBuilder};
use serde_json::Error;
use crate::{common::{append_js_scripts, build_file_map, escape, escapemsg, get_message_data_http, handle_file_request, is_module_request, parse_http_url_path, respond_404}, ipcchannel::IPCResponse, types::{BackendCommand, ChannelMsg, ChildProcess, CommandMessage, NETConnection, NETServer, Responder}};
use crate::types::{Package, ElectricoEvents, Command};

pub struct Backend {
    window:Window,
    package:Package,
    hash:String,
    http_uid:String,
    http_port:u16,
    webview:WebView,
    webviews:HashMap<String, WebView>,
    command_sender:Sender<BackendCommand>,
    command_receiver:Receiver<BackendCommand>,
    child_process:HashMap<String, tokio::sync::mpsc::Sender<ChildProcess>>,
    fs_watcher:HashMap<String, RecommendedWatcher>,
    fs_files:HashMap<i64, File>,
    net_server:HashMap<String, tokio::sync::mpsc::Sender<NETServer>>,
    net_connections:HashMap<String, tokio::sync::mpsc::Sender<NETConnection>>,
    addon_state: HashMap<String, Box<dyn Any>>,
    msg_channels:HashMap<String, tokio::sync::mpsc::Sender<ChannelMsg>>
}

fn handle_electrico_ipc_file(host:String, path:String, src_dir:&PathBuf, backend_js_files: &HashMap<String, Vec<u8>>, sender:tokio::sync::mpsc::Sender<IPCResponse>) {
    trace!("backend file: request {host}:{path}");
    let fpath = path.substring(1, path.len()).to_string();
    let file:PathBuf;
    if fpath.starts_with("/") {
        file = PathBuf::from(fpath.clone());
    } else {
        file = src_dir.join(fpath.clone());
    } 
    trace!("trying load file {}", file.clone().as_mut_os_str().to_str().unwrap());
    handle_file_request(is_module_request(Some(host.as_str())), fpath, file, backend_js_files, crate::types::Responder::HttpProtocol {sender});
}

fn handle_async_electrico_cmd(proxy:EventLoopProxy<ElectricoEvents>, msg:CommandMessage, data_blob:Option<Vec<u8>>) {
    trace!("handle_async_electrico_cmd:{:?}", msg);
    let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:msg.command, responder:Responder::None, data_blob});
}

fn handle_electrico_cmd(proxy:EventLoopProxy<ElectricoEvents>, path:String, query:Option<&str>, request:Vec<u8>, responder:Responder) {
    trace!("backend cmd request {} {}", path, request.len());
    let message_data:Option<(String, Option<Vec<u8>>)> = get_message_data_http(query, request);
    
    if let Some(message_data) = message_data {
        let commandr:Result<Command, Error> = serde_json::from_str(message_data.0.as_str());
        match commandr {
            Ok (command) => {
                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command, responder, data_blob:message_data.1});
            }
            Err(e) => {
                error!("json serialize error {}, {}", e.to_string(), message_data.0);
                respond_404(responder);
                return;
            }
        }
    } else {
        respond_404(responder);
    }
}

#[tokio::main]
pub async fn start_http_server(proxy:EventLoopProxy<ElectricoEvents>, tcp_listener:TcpListener, http_id:&String, src_dir:PathBuf,  backend_js_files: HashMap<String, Vec<u8>>) {
    let listener = tokio::net::TcpListener::from_std(tcp_listener).expect("TcpListener::from_std failed");
    loop {
        let (stream, _) = listener.accept().await.expect("start_http_server listener.accept() failed");
        let io = TokioIo::new(stream);
        let proxy = proxy.clone();
        let http_id = http_id.clone();
        let src_dir = src_dir.clone();
        let backend_js_files = backend_js_files.clone();
        tokio::task::spawn(async move {
            let service = service_fn(|mut request:Request<hyper::body::Incoming>| {
                trace!("http server - got request: {}, {:?}", request.uri().to_string(), request.uri().authority());
                let urlparts = parse_http_url_path(request.uri().path());
                let http_id = http_id.clone();
                let src_dir = src_dir.clone();
                let proxy = proxy.clone();
                let backend_js_files = backend_js_files.clone();
                return async move {
                    if let Some(urlparts) = urlparts {
                        let url = urlparts.3.clone();
                        let protocol = urlparts.1;
                        if urlparts.0 != http_id {
                            error!("http - invalid http id:{}", urlparts.0);
                            return Ok::<_, Infallible>(Response::builder().status(StatusCode::FORBIDDEN).body(Full::new(Bytes::from(StatusCode::FORBIDDEN.to_string()))).expect("http body full failed"));
                        }
                        if protocol == "asyncin" || protocol == "asyncout" {
                            let ws_channel = urlparts.2;
                            //let (ws_sender, mut ws_receiver): (Sender<WebSocketCmd>, Receiver<WebSocketCmd>) = mpsc::channel(1000);
                            let (msg_sender, mut msg_receiver): (tokio::sync::mpsc::Sender<ChannelMsg>, tokio::sync::mpsc::Receiver<ChannelMsg>) = tokio::sync::mpsc::channel(10000);
                            trace!("async websocket:{protocol}");
                            let (resp, fut) = upgrade::upgrade(&mut request).expect("upgrade::upgrade failed");
                            tokio::task::spawn(async move {
                                let socket = fut.await.expect("FragmentCollector failed");
                                let mut ws = fastwebsockets::FragmentCollector::new(socket);
                                let _ = proxy.send_event(ElectricoEvents::BackendConnectWS {channel:ws_channel, msg_sender:msg_sender});
                                if protocol == "asyncin" {
                                    tokio::task::unconstrained(async move {
                                        let mut request_message_data:Option<CommandMessage> = None;
                                        loop {
                                            let proxy = proxy.clone();
                                            if let Ok(frame) =  ws.read_frame().await {
                                                match frame.opcode {
                                                    OpCode::Close => {
                                                        debug!("websocket in closed");
                                                        break;
                                                    },
                                                    OpCode::Binary => {
                                                        trace!("websocket message");
                                                        let req_msg:Vec<u8> = frame.payload.into();
                                                        if let Some(msg) = request_message_data {
                                                            request_message_data = None;
                                                            handle_async_electrico_cmd(proxy, msg, Some(req_msg));
                                                        } else {
                                                            let command_msg:Result<CommandMessage, Error> = serde_json::from_slice(req_msg.as_slice());
                                                            match command_msg {
                                                                Ok (command_msg) => {
                                                                    trace!("backend ipc call {:?}", command_msg.command);
                                                                    if command_msg.data_blob {
                                                                        request_message_data = Some(command_msg);
                                                                    } else {
                                                                        handle_async_electrico_cmd(proxy, command_msg, None);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("handle_async_ws_ipc deserialize:{e}");
                                                                }
                                                            }
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }).await;
                                } else {
                                    loop {
                                        if let Ok(resp_msg) = timeout(Duration::from_millis(1000), msg_receiver.recv()).await {
                                            if let Some(resp_msg) = resp_msg {
                                                trace!("websocket write response message");
                                                let has_data = resp_msg.data_blob.is_some();
                                                let channel = resp_msg.channel;
                                                let params = resp_msg.params;
                                                let message = format!("{has_data}|{channel}|{params}");
                                                let mut buf = BytesMut::with_capacity(message.len());
                                                buf.put(message.as_bytes());
                                                ws.write_frame(Frame::binary(Payload::Bytes(buf))).await.expect("ws.write_frame failed");
                                                if let Some(data) = resp_msg.data_blob {
                                                    let mut buf = BytesMut::with_capacity(data.len());
                                                    buf.put(data.as_slice());
                                                    ws.write_frame(Frame::binary(Payload::Bytes(buf))).await.expect("ws.write_frame data failed");
                                                }
                                            }
                                        }
                                        if let Ok(frame) = timeout(Duration::from_millis(1), ws.read_frame()).await {
                                            if let Ok(frame) = frame {
                                                if frame.opcode == OpCode::Close {
                                                    debug!("websocket out closed");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                            return Ok(Response::builder()
                                .status(hyper::StatusCode::SWITCHING_PROTOCOLS)
                                .header(hyper::header::CONNECTION, "upgrade")
                                .header(hyper::header::UPGRADE, "websocket")
                                .header("Sec-WebSocket-Accept", resp.headers().get("Sec-WebSocket-Accept").expect("Sec-WebSocket-Accept header failed"))
                                .body(Full::new(Bytes::from(""))).expect("http body full failed")); 
                        }
                        let (req_sender, mut req_receiver): (tokio::sync::mpsc::Sender<IPCResponse>, tokio::sync::mpsc::Receiver<IPCResponse>) = tokio::sync::mpsc::channel(1000);
                        let mut known_protocol= false;
                        if protocol == "electrico-file" {
                            known_protocol =true;
                            handle_electrico_ipc_file(urlparts.2, urlparts.3, &src_dir, &backend_js_files, req_sender);
                        } else if protocol == "cmd" {
                            if let Ok(body) = request.body_mut().collect().await {
                                known_protocol =true;
                                handle_electrico_cmd(proxy, urlparts.2, request.uri().query(), body.to_bytes().to_vec(), crate::types::Responder::HttpProtocol {sender:req_sender});
                            }
                        } else {
                            error!("http - no known protocol");
                        }
                        if known_protocol {
                            if let Ok(r) = timeout(Duration::from_secs(300), req_receiver.recv()).await {
                                if let Some(r) = r {
                                    trace!("http - request response: {}", r.params.len());
                                    let r_body = r.params; 
                                    return Ok::<_, Infallible>(Response::builder()
                                        .header("Content-Type", r.mime_type)
                                        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                        .status(r.status)
                                        .body(Full::new(Bytes::from(r_body))).expect("http body full failed"));
                                } else {
                                    error!("http - request no response:{url}");
                                }
                            } else {
                                error!("http - request timed out:{url}");
                            }
                        }
                    }
                    error!("http - invalid http request");
                    return Ok::<_, Infallible>(Response::builder().status(StatusCode::BAD_REQUEST).body(Full::new(Bytes::from(StatusCode::BAD_REQUEST.to_string()))).expect("http body full failed"));
                }
            });
            if let Err(err) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                error!("Error serving connection: {:?}", err);
            }
        });
    }
}

fn create_web_view (
        window:&Window,
        proxy:EventLoopProxy<ElectricoEvents>,
        hash:&String,
        http_uid:&String,
        http_port:u16,
        package:&Package,
        init_script:String) -> WebView {
    let mut is_windows="false";
    #[cfg(target_os = "windows")] {
        is_windows = "true";
    }
    
    let main = package.main.clone();
    let pid = std::process::id();
    debug!("webview:{http_uid},{http_port}");
    let builder = WebViewBuilder::new()
        .with_url(format!("http://{hash}.localhost:{http_port}/{http_uid}@electrico-file/file/{main}-{pid}"))
        .with_devtools(true)
        .with_incognito(false)
        .with_initialization_script(format!(
            "window.__is_windows={is_windows};
            window.__http_protocol = {{'http_port':{http_port}, 'http_uid':'{http_uid}'}};
            {init_script}").as_str());
            
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let webview = builder.build(window).unwrap();
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox).unwrap()
    };
    
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
            //#[cfg(debug_assertions)] {
                window_builder = window_builder.with_inner_size(PhysicalSize::new(1,1));
            //}
            /*#[cfg(not(debug_assertions))] {
                window_builder = window_builder.with_visible(false);
            }*/
        }
        #[cfg(not(target_os = "macos"))] {
            window_builder = window_builder.with_visible(false);
        }

        let window = window_builder
            .build(event_loop)
            .unwrap();
        
        let mut hasher = DefaultHasher::new();
        format!("{}/{}", src_dir.as_os_str().to_str().unwrap(), package.main.to_string().as_str()).hash(&mut hasher);
        let hash = format!("{}", hasher.finish());
        let http_uid = Uuid::new_v4().to_string();
        let listener= TcpListener::bind("127.0.0.1:0").expect("start_http_server - TcpListener failed");
        let addr = listener.local_addr().expect("listener.local_addr failed");
        let _ = listener.set_nonblocking(true);
        let s_proxy = proxy.clone();
        let s_http_uid = http_uid.clone();
        let s_src_dir = src_dir.clone();
        let s_backend_js_files = backend_js_files.clone();
        thread::spawn(move || {
           start_http_server(s_proxy, listener, &s_http_uid, s_src_dir, s_backend_js_files);
        });
        
        let webview = create_web_view(&window, proxy, &hash,  &http_uid, addr.port(), &package, init_script);
        
        Backend {
            window:window,
            webview:webview,
            webviews:HashMap::new(),
            package:package,
            hash:hash,
            http_uid:http_uid,
            http_port:addr.port(),
            command_sender,
            command_receiver,
            child_process: HashMap::new(),
            fs_watcher: HashMap::new(),
            fs_files: HashMap::new(),
            net_server: HashMap::new(),
            net_connections: HashMap::new(),
            addon_state: HashMap::new(),
            msg_channels:HashMap::new()
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
    pub fn call_ipc_channel(&mut self, browser_window_id:String, request_id:String, channel: String, params:String, data_blob:Option<Vec<u8>>) {
        trace!("call_ipc_channel {} {}", &request_id, &params);
        let args = format!("{{\"browser_window_id\":\"{browser_window_id}\", \"request_id\":\"{request_id}\", \"params\":\"{}\"}}", escapemsg(&params));
        if let Some(sender) = self.msg_channels.get(&request_id) {
            // send to remote window
            debug!("send to remote window");
            let _ = sender.blocking_send(ChannelMsg {channel, params:args, data_blob:data_blob});
        } else {
            self.send_channel_message(format!("ipc_{channel}"), args, data_blob);
        }
    }
    pub fn connect_ws(&mut self, channel:String, msg_sender:tokio::sync::mpsc::Sender<ChannelMsg>) {
        self.msg_channels.insert(channel.clone(), msg_sender);
        self.send_channel_message("ipc_connect".to_string(), channel, None)
    }
    pub fn send_channel_message(&mut self, channel:String, args:String, data:Option<Vec<u8>>) {
        if let Some(sender) = self.msg_channels.get("ipcout") {
            let _ = sender.blocking_send(ChannelMsg {channel, params:args, data_blob:data});
        } else {
            error!("send_channel_message - ipcout websocket not there");
        }
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
            let _ = sender.blocking_send(ChildProcess::StdinWrite {data, end});
            }
        } else {
            trace!("child_process_callback {} {}", stream, pid);
            self.send_channel_message(format!("cp_data_{pid}"), stream, data);
        }
    }
    pub fn child_process_exit(&mut self, pid:String, exit_code:Option<i32>) {
        let args:String;
        if let Some(exit_code) = exit_code {
            args=format!("{exit_code}");
        } else {
            args=format!("");
        }
        self.send_channel_message(format!("cp_exit_{pid}"), args, None);
    }
    pub fn fs_watch_callback(&mut self, wid:String, event:Event) {
        let args = format!("{{\"kind\":\"{:?}\", \"filenames\":\"{}\"}}",
                event.kind,
                escapemsg(&event.paths.iter().map(|x| x.as_os_str().to_str().unwrap()).collect::<Vec<_>>().join(";")));
        self.send_channel_message(format!("fsw_{wid}"), args, None);
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
            let _ = sender.blocking_send(ChildProcess::Disconnect);
        }
    }
    pub fn child_process_kill(&mut self, pid:String) {
        trace!("child_process_kill {}", pid);
        if let Some(sender) = self.child_process.get(&pid) {
            let sender = sender.clone();
            let _ = sender.blocking_send(ChildProcess::Kill);
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
        self.net_connections.insert(id.clone(), sender.clone());
        debug!("net_server_conn_start:{}", self.net_connections.len());
        self.send_channel_message(format!("net_start_{id}"), hook, None);
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
        self.send_channel_message(format!("net_data"), id, data);
    }
    pub fn net_connection_end(&mut self, id:String) {
        if let Some(sender) = self.net_connections.get(&id) {
            let _ = sender.send(NETConnection::Disconnect);
            self.net_connections.remove(&id);
        }
        self.send_channel_message(format!("net_end"), id, None);
    }
    pub fn net_write_connection(&mut self, id:String, end:bool, data:Vec<u8>) {
        if let Some(sender) = self.net_connections.get(&id) {
            let sender = sender.clone();
            let _ = sender.blocking_send(NETConnection::Write { data, end});
            if end {
                self.net_connections.remove(&id);
            }
        } else {
            error!("net_write_connection no sender for id {}", id);
        }
    }
    pub fn net_set_timeout(&mut self, id:String, timeout:u128) {
        if let Some(sender) = self.net_connections.get(&id) {
            let sender = sender.clone();
            let _ = sender.blocking_send(NETConnection::SetTimeout { timeout:Some(timeout) });
        } else {
            error!("net_set_timeout no sender for id {}", id);
        }
    }
    pub fn execute_sync(&mut self, proxy:EventLoopProxy<ElectricoEvents>, script:String) -> String {
        let (_, backendjs) = backend_resources(&self.package);
        let init_script = format!("window._no_websocket=true;\n{}\nwindow.__electrico.loadMain();", backendjs);
        let webview = create_web_view(&self.window, proxy, &self.hash, &self.http_uid, self.http_port, &self.package, init_script);
        let uuid = Uuid::new_v4().to_string();
        let _ = webview.evaluate_script(format!("{script}.then(r=>{{$e_node.syncExecuteSyncResponse({{'uuid':'{uuid}', 'data':r+''}});}}).catch(e=>{{$e_node.syncExecuteSyncResponse({{'uuid':'{uuid}', 'error':e+''}});}});").as_str());
        self.webviews.insert(uuid.clone(), webview);
        return uuid;
    }
    pub fn execute_sync_response(&mut self, uuid:String, data:Option<String>, error:Option<String>) {
        self.webviews.remove(&uuid);
        self.addon_state_insert(&uuid, (data, error)); 
    }
    pub fn process_commands(&mut self) {
        if let Ok(command) = self.command_receiver.try_recv() {
            match command {
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