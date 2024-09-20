use std::{path::PathBuf, sync::mpsc::{self, Receiver, Sender}};
use muda::MenuId;
use substring::Substring;
use log::{debug, error, trace};
use include_dir::{include_dir, Dir};
use tao::{dpi::PhysicalSize, event_loop::{EventLoop, EventLoopProxy}, window::{Window, WindowBuilder}};
use wry::{http::Request, RequestAsyncResponder, WebView, WebViewBuilder};
use serde_json::Error;
use crate::{common::{append_js_scripts, build_file_map, escape, handle_file_request, is_module_request}, ipcchannel::IPCMsg, types::BackendCommand};
use crate::types::{Package, ElectricoEvents, Command};

pub struct Backend {
    _window:Window,
    webview:WebView,
    retry_sender:Sender<BackendCommand>,
    retry_receiver:Receiver<BackendCommand>
}

impl Backend {
    pub fn new(src_dir:PathBuf, package:&Package, event_loop:&EventLoop<ElectricoEvents>, proxy:EventLoopProxy<ElectricoEvents>) -> Backend {
        let (retry_sender, retry_receiver): (Sender<BackendCommand>, Receiver<BackendCommand>) = mpsc::channel();

        let mut backendjs:String = String::new();
        const JS_DIR_SHARED: Dir = include_dir!("src/js/shared");
        backendjs = append_js_scripts(backendjs, JS_DIR_SHARED);
        const JS_DIR_BACKEND: Dir = include_dir!("src/js/backend");
        backendjs = append_js_scripts(backendjs, JS_DIR_BACKEND);
        let backend_js_files = build_file_map(&JS_DIR_BACKEND);
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
        
        let cmd_handler = move |request: Request<Vec<u8>>, responder:RequestAsyncResponder| {
            trace!("backend cmd request {} {}", request.uri().path().to_string(), request.body().len());
            let msgr =  String::from_utf8(request.body().to_vec());
            match msgr {
                Ok(msg) => {
                  trace!("backend cmd request body {}", msg.as_str());
                  let commandr:Result<Command, Error> = serde_json::from_str(msg.as_str());
                  match commandr {
                    Ok (command) => {
                      let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command, responder});
                    }
                    Err(e) => {
                      error!("json serialize error {}", e.to_string());
                    }
                  }
                },
                Err(e) => {
                  error!("utf8 error {}", e.to_string());
                }
            }
        };
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(5).enable_io().enable_time().build().unwrap();
        
        let fil_handler = move |request: Request<Vec<u8>>, responder:RequestAsyncResponder| {
            let fil_src_dir = src_dir.clone();
            trace!("backend fil: request {}", request.uri().path().to_string());
            let fpath = request.uri().path().to_string().as_str().substring(1, request.uri().path().to_string().as_str().len()).to_string();
            
            let file = fil_src_dir.join(fpath.clone());
            trace!("trying load file {}", file.clone().as_mut_os_str().to_str().unwrap());
            handle_file_request(&tokio_runtime, is_module_request(request.uri().host()), fpath, file, &backend_js_files, responder);
        };
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
        let builder = WebViewBuilder::new(&window);
    
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

        let webview = builder
            .with_html("<h1>Electrico Node Backend</h1>")
            .with_asynchronous_custom_protocol("fil".into(), fil_handler)
            .with_asynchronous_custom_protocol("cmd".into(), cmd_handler)
            .with_devtools(true)
            .with_incognito(true)
            .with_initialization_script(("window.__is_windows=".to_string()+is_windows+";"+init_script.as_str()).as_str())
            .build().unwrap();
          
        #[cfg(debug_assertions)]
        webview.open_devtools();
        Backend {
            _window:window,
            webview:webview,
            retry_sender,
            retry_receiver
        }
    }
    pub fn command_callback(&mut self, command:String, message:String) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.callback['{}']('{}')", command, message).as_str());
    }
    pub fn call_ipc_channel(&mut self, browser_window_id:&String, request_id:&String, params:String, sender:Sender<IPCMsg>) {
         let request_id2 = request_id.clone();
         trace!("call_ipc_channel {} {}", &request_id2, &params);
         _ = self.webview.evaluate_script_with_callback(
            format!("window.__electrico.callIPCChannel('{}@{}@@{}');", browser_window_id, request_id, escape(&params)).as_str()
            , move |r| {
                if r.len()>0 {
                    let _ = sender.send(IPCMsg::Called);
                    trace!("call_ipc_channel OK {}", &request_id2);
                } else {
                    trace!("call_ipc_channel not OK {}", &request_id2);
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
    pub fn window_all_closed(&mut self) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.callAppOn('window-all-closed');").as_str());
    }
    pub fn menu_selected(&mut self, id:MenuId) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.menuSelected('{}');", id.as_ref()).as_str());
    }
    pub fn dom_content_loaded(&mut self, id:&String) {
        let _ = self.webview.evaluate_script(format!("window.__electrico.domContentLoaded('{}');", id).as_str());
    }
    pub fn child_process_callback(&mut self, pid:String, stream:String, data:String) {
        trace!("child_process_callback {}", data);
        let retry_sender = self.retry_sender.clone();
        let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{window.__electrico.child_process.callback.on_{}('{}', '{}');}});", stream, pid, escape(&data).as_str()), move |r| {
            if r.len()==0 {
                trace!("child_process_callback not OK - resending");
                let _ = retry_sender.send(BackendCommand::ChildProcessCallback { pid:pid.clone(), stream:stream.clone(), data:data.clone() });
            }
        });
    }
    pub fn child_process_exit(&mut self, pid:String, exit_code:Option<i32>) {
        let call_script:String;
        if let Some(exit_code) = exit_code {
            call_script=format!("window.__electrico.child_process.callback.on_close('{}', {});", pid, exit_code.to_string());
        } else {
            call_script=format!("window.__electrico.child_process.callback.on_close('{}');", pid);
        }
        let retry_sender = self.retry_sender.clone();
        let _ = self.webview.evaluate_script_with_callback(&format!("window.__electrico.call(()=>{{{}}});", call_script.as_str()), move |r| {
            if r.len()==0 {
                trace!("child_process_exit not OK - resending");
                let _ = retry_sender.send(BackendCommand::ChildProcessExit { pid: pid.clone(), exit_code: exit_code.clone() });
            }
        });
    }
    pub fn retry_commands(&mut self) {
        if let Ok(command) = self.retry_receiver.try_recv() {
            match command {
                BackendCommand::ChildProcessCallback { pid, stream, data } => {
                    trace!("retrying ChildProcessCallback");
                    self.child_process_callback(pid, stream, data);
                }
                BackendCommand::ChildProcessExit { pid, exit_code } => {
                    trace!("retrying ChildProcessExit");
                    self.child_process_exit(pid, exit_code);
                }
            }
        }
    }
}