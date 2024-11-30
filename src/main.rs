mod common;
mod types;
mod ipcchannel;
mod backend;
mod frontend;
mod node;
mod electron;
use std::{fs::{self}, io::{self, Write}, path::{Path, PathBuf}, str::FromStr, sync::mpsc::{self, Receiver, Sender}, time::{Duration, SystemTime}};

use electron::electron::process_electron_command;
use env_logger::Env;
use json_comments::StripComments;
use muda::{Menu, MenuEvent};
use serde_json::Error;
use backend::Backend;
use frontend::Frontend;
use ipcchannel::{IPCChannel, IPCResponse};
use log::{debug, error, info, trace, warn};
use node::node::{process_node_command, AppEnv};
use reqwest::StatusCode;
use tao::event_loop::EventLoopBuilder;
use common::{build_file_map, escape, handle_file_request, read_file, respond_404, respond_ok, respond_status, CONTENT_TYPE_BIN, CONTENT_TYPE_HTML, CONTENT_TYPE_JSON, JS_DIR_FRONTEND};
use tempfile::TempDir;
use types::{Command, ElectricoEvents, ForkParams, Package, Resources};
use tao::{event::{Event, StartCause, WindowEvent},event_loop::{ControlFlow, EventLoop}};
use uuid::Uuid;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    param: Option<String>,
    #[arg(short, long)]
    fork: Option<String>,
}

fn main() -> wry::Result<()> {
  let env = Env::default()
        .filter_or("LOG_LEVEL", "debug")
        .write_style_or("LOG_STYLE", "always");

  env_logger::init_from_env(env);
  
  let tokio_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(50).enable_io().enable_time().build().unwrap();
  
  let mut rsrc_dir:PathBuf;
  let package:Package;
  let args = Args::parse();
  let mut _tmpdir_h:Option<TempDir> = None;
  let mut add_args: Vec<String> = Vec::new();
  if let Some(p) = args.param {
    package = Package::new("shell.js".to_string(), "1".to_string(), "shell".to_string());
    let tmpdir:TempDir = tempfile::tempdir().unwrap();
    let mut tmppath = PathBuf::from(tmpdir.path());
    _tmpdir_h = Some(tmpdir);
    rsrc_dir = tmppath.clone();
    tmppath.push("shell.js");
    trace!("tmp path: {}", tmppath.as_os_str().to_str().unwrap());
    let _ = fs::write(tmppath, format!("
        let r = eval('{}'); 
        let req = window.createCMDRequest(true, 'ShellCallback');
        req.send(JSON.stringify({{action:'ShellCallback', 'stdout':r+''}}));
        ", escape(&p)));
  } else if let Some(f) = args.fork {
    trace!("fork: {}", f);
    let fork_params:ForkParams = serde_json::from_str(f.as_str()).expect("Can't deserialize fork parameter json");
    add_args.append(&mut fork_params.args.clone());
    package = Package::new_fork(fork_params.module_main, "1".to_string(), "fork".to_string(), fork_params.hook, fork_params.clientid, fork_params.env);
    rsrc_dir = PathBuf::from(fork_params.module_src);
  } else {
    rsrc_dir = std::env::current_exe()
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
    package = serde_json::from_str(packagetxt.as_str()).expect("Can't deserialize package.json");
    trace!("package.json main js: {}", package.main.as_str());
  }
  let mut app_env = AppEnv::new(rsrc_dir.as_os_str().to_str().unwrap().to_string(), &mut add_args);

  let frontend_js_files = build_file_map(&JS_DIR_FRONTEND);

  let event_loop:EventLoop<ElectricoEvents> = EventLoopBuilder::with_user_event().build();
  let proxy: tao::event_loop::EventLoopProxy<ElectricoEvents> = event_loop.create_proxy();
  
  let mut backend = Backend::new(rsrc_dir.clone(), &package, &event_loop, proxy.clone());
  let mut frontend = Frontend::new(rsrc_dir.clone());
  let mut ipc_channel = IPCChannel::new();
  
  let menu_channel = MenuEvent::receiver();
  
  let mut _main_menu_hold:Option<Menu> = None;
  
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
      Event::UserEvent(ElectricoEvents::ExecuteCommand{command, responder, data_blob}) => {
        trace!("backend ExecuteCommand call");
        match command {
          Command::PostIPC { browser_window_id, request_id, params} => {
            trace!("PostIPC {} {} {}", browser_window_id, request_id, params);
            let r_request_id = request_id.clone();
            
            let (sender, receiver): (Sender<IPCResponse>, Receiver<IPCResponse>) = mpsc::channel();
            ipc_channel.start(request_id.clone(), sender.clone());
            tokio_runtime.spawn(
              async move {
                let ipc_response = receiver.recv_timeout(Duration::from_secs(600));
                match ipc_response {
                  Ok (response) => {
                    trace!("PostIPC Response {}", response.params.len());
                    respond_status(StatusCode::OK, response.mime_type, response.params, responder);
                  },
                  Err (_e) => {
                    warn!("PostIPC request expired (timeout): {}", r_request_id.clone());
                    respond_status(StatusCode::GONE, CONTENT_TYPE_JSON.to_string(), "expired (timeout)".to_string().into_bytes(), responder);
                  }
                }
              }
            );
            backend.call_ipc_channel(browser_window_id, request_id, params, data_blob);
          },
          Command::SetIPCResponse {request_id, file_path} => {
            trace!("backend ExecuteCommand call SetIPCResponse {}", request_id);
            match ipc_channel.get(&request_id) {
              Some(sender) => {
                if let Some(file_path) = file_path {
                  let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream().to_string();
                  if let Some(data) = read_file(&file_path) {
                    let _ = sender.send(IPCResponse::new(data, mime_type));
                  } else {
                    respond_404(responder);
                    return;
                  }
                } else {
                  if let Some(params) = data_blob {
                    let _ = sender.send(IPCResponse::new(params, CONTENT_TYPE_JSON.to_string()));
                  } else {
                    error!("SetIPCResponse - no data blob");
                    respond_404(responder);
                    return;
                  }
                }
                ipc_channel.end(&request_id);
                respond_ok(responder);
              },
              None => {
                warn!("ipc_channel - backend ExecuteCommand call SetIPCResponse request expired (timeout): {}", request_id);
                respond_ok(responder);
              }
            }
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
          },
          Command::ShellCallback { stdout } => {
            debug!("ShellCallback {}", stdout);
            let _ = io::stdout().write_all(stdout.as_bytes());
            let _ = io::stdout().flush();
            *control_flow = ControlFlow::Exit;
          },
          Command::FrontendGetDataBlob { id } => {
            if let Some(data) = frontend.get_data_blob(id) {
              respond_status(StatusCode::OK, CONTENT_TYPE_BIN.to_string(), data, responder);
          } else {
              respond_404(responder);
          }
          }
          Command::Electron { invoke } => {
            let menu_ret = process_electron_command(&tokio_runtime, event_loop, proxy.clone(), 
              &mut app_env, &rsrc_dir, &package,
              &mut frontend, &mut backend, invoke, responder, data_blob);
            if let Some(menu) = menu_ret.clone() {
              _main_menu_hold = menu_ret;
            }
          },
          Command::Node { invoke } => {
            process_node_command(&tokio_runtime, &app_env, proxy.clone(), &mut backend, invoke, responder, data_blob);
          }
        }
      },
      Event::UserEvent(ElectricoEvents::SendChannelMessageRetry { browser_window_id, rid, channel, args }) => {
        trace!("SendChannelMessageRetry");
        frontend.send_channel_message(proxy.clone(), browser_window_id, rid, channel, args, None);
      },
      Event::UserEvent(ElectricoEvents::Exit) => {
        backend.shutdown();
        *control_flow = ControlFlow::Exit;
      },
      _ => (),
    }
  });
}