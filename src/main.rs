mod common;
mod types;
mod ipcchannel;
mod backend;
mod frontend;
mod node;
mod electron;
use std::{collections::HashMap, fs, path::{Path, PathBuf}, str::FromStr, sync::mpsc::{self, Receiver, Sender}, time::{Duration, SystemTime}};
use electron::electron::process_electron_command;
use env_logger::Env;
use json_comments::StripComments;
use muda::{Menu, MenuEvent};
use serde_json::Error;
use backend::Backend;
use frontend::Frontend;
use ipcchannel::{IPCChannel, IPCMsg};
use log::{debug, error, trace, warn};
use node::node::{process_node_command, AppEnv};
use reqwest::StatusCode;
use tao::event_loop::EventLoopBuilder;
use common::{build_file_map, handle_file_request, respond_ok, respond_status, CONTENT_TYPE_HTML, CONTENT_TYPE_JSON, JS_DIR_FRONTEND};
use types::{Command, ElectricoEvents, Package, Resources};
use tao::{event::{Event, StartCause, WindowEvent},event_loop::{ControlFlow, EventLoop}};

fn main() -> wry::Result<()> {
  let env = Env::default()
        .filter_or("LOG_LEVEL", "debug")
        .write_style_or("LOG_STYLE", "always");

  env_logger::init_from_env(env);

  let tokio_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(20).enable_io().enable_time().build().unwrap();

  let mut rsrc_dir = std::env::current_exe()
    .expect("Can't find path to executable");

  while rsrc_dir.pop() {
    let rsrc_link_dir = rsrc_dir.join("ResourcesLink.json");
    let rsrc_link = Path::new(&rsrc_link_dir);
    if rsrc_link.exists() {
      if let Ok(rsrc_link_str) = fs::read_to_string(rsrc_link) {
        let rsrc_link_json = StripComments::new(rsrc_link_str.as_bytes());
        let res:Result<Resources, Error> = serde_json::from_reader(rsrc_link_json);
        if let Ok(res) = res {
          if let Some(link) = res.link {
            trace!("link {}", link);
            rsrc_dir = PathBuf::from_str(link.as_str()).unwrap();
            break;
          }
        }
      }
    }
    rsrc_dir.push("Resources");
    if rsrc_dir.exists() && rsrc_dir.is_dir() {
      break;
    }
    rsrc_dir.pop();
  }

  let pgk_file = rsrc_dir.join("package.json");
  trace!("package.json path: {}", pgk_file.as_path().as_os_str().to_str().unwrap());
  
  let packagetxt = std::fs::read_to_string(pgk_file).expect("Can't find package.json");
  let package:Package = serde_json::from_str(packagetxt.as_str()).expect("Can't deserialize package.json");
  trace!("package.json main js: {}", package.main.as_str());

  let frontend_js_files = build_file_map(&JS_DIR_FRONTEND);

  let event_loop:EventLoop<ElectricoEvents> = EventLoopBuilder::with_user_event().build();
  let proxy: tao::event_loop::EventLoopProxy<ElectricoEvents> = event_loop.create_proxy();
  
  let mut backend = Backend::new(rsrc_dir.clone(), &package, &event_loop, proxy.clone());
  let mut frontend = Frontend::new(rsrc_dir.clone());
  let mut ipc_channel = IPCChannel::new();
  
  let menu_channel = MenuEvent::receiver();
  
  let mut _main_menu_hold:Option<Menu> = None;
  let mut app_env = AppEnv::new(rsrc_dir.as_os_str().to_str().unwrap().to_string());

  event_loop.run( move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;
    if let Ok(event) = menu_channel.try_recv() {
      if event.id == "quit" {
        backend.window_close(&None);
        return;
      } else if event.id == "toggleDevTools" {
        frontend.toggle_dev_tools();
        return;
      }
      backend.menu_selected(event.id);
    }
    backend.process_commands();
    match event {
      Event::NewEvents(StartCause::Init) => {
        
      },
      Event::Opened{ urls } => {
        for url in urls{
          app_env.add_arg(url.as_str().to_string());
        }
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
        ..
      } => {
        if let Some(w_id) = frontend.get_id(&window_id) {
          backend.window_close(&Some(w_id.to_string()));
        }
      },
      Event::UserEvent(ElectricoEvents::FrontendNavigate{browser_window_id, page, preload}) => {
        
      },
      Event::UserEvent(ElectricoEvents::IPCCallRetry{browser_window_id, request_id, params, sender}) => {
        backend.call_ipc_channel(&browser_window_id, &request_id, params, sender);
      }
      Event::UserEvent(ElectricoEvents::ExecuteCommand{command, responder}) => {
        trace!("backend ExecuteCommand call");
        match command {
          Command::PostIPC { browser_window_id, request_id, params} => {
            trace!("PostIPC {} {} {}", browser_window_id, request_id, params);
            let r_request_id = request_id.clone();
            
            let (sender, receiver): (Sender<IPCMsg>, Receiver<IPCMsg>) = mpsc::channel();
            ipc_channel.start(request_id.clone(), sender.clone());

            let callipc_proxy = proxy.clone();
            let callipc_browser_window_id = browser_window_id.clone();
            let callipc_request_id = request_id.clone();
            let callipc_params = params.clone();
            let callipc_sender = sender.clone();
            let started = SystemTime::now();
            let mut called = false;
            tokio_runtime.spawn(
              async move {
                while !called {
                  let called_response = receiver.recv_timeout(Duration::from_millis(100));
                  match called_response {
                    Ok (response) => {
                      match response {
                        IPCMsg::Called => {
                          called = true;
                        },
                        IPCMsg::Response { params } => {
                          respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), params.to_string().into_bytes(), responder);
                          return;
                        }
                      }
                    },
                    Err (_e) => {
                      match started.elapsed() {
                        Ok(elapsed) => {
                            if elapsed.as_secs()>600 {
                                warn!("PostIPC Call Expired {}", callipc_request_id.clone());
                                respond_status(StatusCode::GONE, CONTENT_TYPE_JSON.to_string(), "call expired (timeout)".to_string().into_bytes(), responder);
                                return;
                            }
                        },
                        Err(e) => {
                            error!("PostIPC SystemTimeError {}", e.to_string());
                        }
                      }
                      let _ = callipc_proxy.send_event(
                        ElectricoEvents::IPCCallRetry { browser_window_id:callipc_browser_window_id.clone(), request_id:callipc_request_id.clone(), params:callipc_params.clone(), sender:callipc_sender.clone() }
                      );
                    }
                  }
                }
                let ipc_response = receiver.recv_timeout(Duration::from_secs(600));
                match ipc_response {
                  Ok (response) => {
                    match response {
                      IPCMsg::Response { params } => {
                        trace!("PostIPC Response {}", params);
                        respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), params.to_string().into_bytes(), responder);
                      },
                      _ => ()
                    }
                  },
                  Err (_e) => {
                    warn!("PostIPC request expired (timeout): {}", r_request_id.clone());
                    respond_status(StatusCode::GONE, CONTENT_TYPE_JSON.to_string(), "expired (timeout)".to_string().into_bytes(), responder);
                  }
                }
              }
            );
            backend.call_ipc_channel(&browser_window_id, &request_id, params, sender);
          },
          Command::SetIPCResponse {request_id, params} => {
            trace!("backend ExecuteCommand call SetIPCResponse {} {}", request_id, params);
            match ipc_channel.get(&request_id) {
              Some(sender) => {
                let _ = sender.send(IPCMsg::Response { params });
                ipc_channel.end(&request_id);
              },
              None => {
                warn!("ipc_channel - backend ExecuteCommand call SetIPCResponse request expired (timeout): {}", request_id);
              }
            }
            
            respond_ok(responder);
          },
          Command::BrowserWindowReadFile { browser_window_id, file_path, module } => {
              trace!("BrowserWindowReadFile {} {}", browser_window_id, file_path);
              match frontend.get_client_path_base(&browser_window_id) {
                  Some(client_path_base) => {
                    let file = rsrc_dir.join(file_path.clone());
                    if !file.starts_with(client_path_base) {
                        error!("browser client access to file forbidden: {} {}", file.as_os_str().to_str().unwrap(), client_path_base.as_os_str().to_str().unwrap());
                        respond_status(StatusCode::FORBIDDEN, CONTENT_TYPE_HTML.to_string(), "forbidden".to_string().into_bytes(), responder);
                        return;
                    }  
                    handle_file_request(&tokio_runtime, module, file_path, file, &frontend_js_files, responder);
                  },
                  None => {
                      error!("browser client access to file forbidden - no client_path_base: {}", file_path);
                      respond_status(StatusCode::FORBIDDEN, CONTENT_TYPE_HTML.to_string(), "forbidden".to_string().into_bytes(), responder);
                  }
              }
          },
          Command::DOMContentLoaded { browser_window_id, title} => {
            backend.dom_content_loaded(&browser_window_id);
            frontend.dom_content_loaded(&browser_window_id, title);
          }
          Command::Electron { invoke } => {
            let menu_ret = process_electron_command(&tokio_runtime, event_loop, proxy.clone(), 
              &mut app_env, &rsrc_dir, &package,
              &mut frontend, &mut backend, invoke, responder);
            if let Some(menu) = menu_ret.clone() {
              _main_menu_hold = menu_ret;
            }
          }
          Command::Node { invoke } => {
            process_node_command(&tokio_runtime, &app_env, proxy.clone(), &mut backend, invoke, responder);
          }
        }
      },
      Event::UserEvent(ElectricoEvents::SendChannelMessageRetry { browser_window_id, channel, args }) => {
        trace!("SendChannelMessageRetry");
        frontend.send_channel_message(proxy.clone(), browser_window_id, channel, args);
      },
      Event::UserEvent(ElectricoEvents::Exit) => {
        backend.shutdown();
        *control_flow = ControlFlow::Exit;
      },
      _ => (),
    }
  });
}