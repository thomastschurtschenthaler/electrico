use std::{collections::HashMap, env, thread, time::Duration};
use log::{debug, error, info, trace, warn};
use reqwest::StatusCode;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::webview_version;

use crate::{backend::Backend, common::{respond_ok, respond_status, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, node::types::{Process, ProcessVersions}, types::{ElectricoEvents, Responder}};
use super::{apis, addons, types::{ConsoleLogLevel, NodeCommand}};

pub struct AppEnv {
    pub start_args: Vec<String>,
    pub app_name:Option<String>,
    pub resources_path:String
}

impl AppEnv {
    pub fn new(resources_path:String, add_args:&mut Vec<String>) -> AppEnv {
        let mut args = Vec::new();
        for arg in std::env::args() {
            args.push(arg);
        }
        args.append(add_args);
        AppEnv {
            start_args:args,
            app_name:None,
            resources_path:resources_path
        }
    }
    pub fn add_arg(&mut self, arg:String) {
        self.start_args.push(arg);
    }
}

pub fn process_node_command(tokio_runtime:&Runtime, app_env:&AppEnv,
        proxy:EventLoopProxy<ElectricoEvents>,
        backend:&mut Backend,
        command:NodeCommand,
        responder:Responder,
        data_blob:Option<Vec<u8>>)  {
    match command {
        NodeCommand::ConsoleLog { params } => {
            match params.level {
                ConsoleLogLevel::Info => {
                    info!("{} {}", params.logmsg, params.logdata.or(Option::Some("".to_string())).unwrap());
                },
                ConsoleLogLevel::Debug => {
                    debug!("{} {}", params.logmsg, params.logdata.or(Option::Some("".to_string())).unwrap());
                },
                ConsoleLogLevel::Warn => { 
                    warn!("{} {}", params.logmsg, params.logdata.or(Option::Some("".to_string())).unwrap());
                },
                ConsoleLogLevel::Error => {
                    error!("{} {}", params.logmsg, params.logdata.or(Option::Some("".to_string())).unwrap());
                },
                ConsoleLogLevel::Trace => {
                    trace!("{} {}", params.logmsg, params.logdata.or(Option::Some("".to_string())).unwrap());
                }
            }
            respond_ok(responder);
        },
        NodeCommand::GetProcessInfo => {
            trace!("node process_info");
            if let Some(json) = process_info(app_env) {
                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
            } else {
                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("GetProcessInfo error").into_bytes(), responder);
            }
        },
        NodeCommand::GetStartArgs => {
            trace!("node GetStartArgs");
            match serde_json::to_string(&app_env.start_args) {
                Ok(json) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("GetStartArgs json serialization error: {}", e).into_bytes(), responder);
                }
            }
        },
        NodeCommand::ExecuteSync { script } => {
            let id = backend.execute_sync(proxy, script);
            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), id.as_bytes().to_vec(), responder);
        },
        NodeCommand::WaitExecuteSync { id, duration } => {
            let resp:Option<&mut(Option<String>, Option<String>)> = backend.addon_state_get_mut(&id);
            if let Some(resp) = resp {
                if let Some(r) = &resp.0 {
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), r.as_bytes().to_vec(), responder);
                } else if let Some(e) = &resp.1 {
                    respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), e.as_bytes().to_vec(), responder);
                } else {
                    respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), "no data".as_bytes().to_vec(), responder);
                }
            } else {
                tokio_runtime.spawn(
                    async move {
                        thread::sleep(Duration::from_millis(duration));
                        respond_status(StatusCode::NOT_FOUND, CONTENT_TYPE_TEXT.to_string(), "busy".as_bytes().to_vec(), responder);
                    }
                );
                return;
            }
            backend.addon_state_remove(&id);
        },
        NodeCommand::ExecuteSyncResponse { uuid, data, error } => {
            respond_ok(responder);
            backend.execute_sync_response(uuid, data, error);
        },
        NodeCommand::ChannelSendMessage { channel, args } => {
            respond_ok(responder);
            backend.send_channel_message(channel, args, data_blob);
        }
        NodeCommand::Api { data } => {
            apis::apis::process_command(tokio_runtime, app_env, proxy, backend, data, responder, data_blob); 
        },
        NodeCommand::Addon { data } => {
            addons::addons::process_command(tokio_runtime, app_env, proxy, backend, data, responder, data_blob); 
        }
    }
}

fn process_info(app_env:&AppEnv) -> Option<String> {
    trace!("node process_info");
    const ELECTRICO_VERSION: &str = env!("CARGO_PKG_VERSION");
    let webview_version:String = webview_version().expect("webview_version failed");

    let chrome = format!("WebView-{}", webview_version);
    let node = format!("Electrico-{}/{}", ELECTRICO_VERSION, webview_version);
    let electron = format!("Electrico-{}", ELECTRICO_VERSION);

    let mut platform = "win32";
    #[cfg(target_os = "macos")] {
        platform = "darwin";
    }
    #[cfg(target_os = "linux")] {
        platform = "linux";
    }
    let mut node_env="production".to_string();
    let mut electron_is_dev="0".to_string();

    let mut home = "".to_string();
    if let Some(user_dirs) = directories::UserDirs::new() {
        home = user_dirs.home_dir().as_os_str().to_str().unwrap().to_string();
    }

    #[cfg(debug_assertions)] {
        node_env="development".to_string();
        electron_is_dev="1".to_string();
    }
    let mut exec_path = "".to_string();
    if let Ok(p) = std::env::current_exe() {
        exec_path = p.as_os_str().to_str().unwrap().to_string();
    }
    let mut env:HashMap<String, String> = HashMap::new();
    for (k, v) in env::vars() {
        env.insert(k, v);
    }
    env.insert("NODE_ENV".to_string(), node_env);
    env.insert("ELECTRON_IS_DEV".to_string(), electron_is_dev);

    let process_info = Process::new(platform.to_string(), 
        ProcessVersions::new(node, chrome, electron), 
        env,
        app_env.resources_path.clone(),
        exec_path,
        app_env.start_args.clone(),
        std::process::id()
    );
    match serde_json::to_string(&process_info) {
        Ok(json) => {
            return Some(json);
        },
        Err(e) => {
            error!("process_info json error: {e}");
            return None;
        }
    }
}