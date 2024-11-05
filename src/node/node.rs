use std::{env, fs::{self, OpenOptions}, io::{Read, Seek, SeekFrom, Write}, path::Path, time::SystemTime};
use log::{debug, error, info, trace, warn};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use reqwest::{header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE}, Method, Request, StatusCode, Url};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::{http::Response, webview_version, RequestAsyncResponder};

use crate::{backend::Backend, common::{respond_404, respond_client_error, respond_ok, respond_status, CONTENT_TYPE_BIN, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, node::{common::send_command, ipc::{ipc_connection, ipc_server}, types::{FSDirent, Process, ProcessEnv, ProcessVersions}}, types::{BackendCommand, ElectricoEvents}};
use super::{addons::addons::process_command, process::child_process_spawn, types::{ConsoleLogLevel, FSStat, NodeCommand}};

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
        responder:RequestAsyncResponder,
        data_blob:Option<Vec<u8>>)  {
    let command_sender = backend.command_sender();
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
            let path = env::var("PATH").unwrap();
            
            let process_info = Process::new(platform.to_string(), 
                ProcessVersions::new(node, chrome, electron), 
                ProcessEnv::new(node_env, electron_is_dev, home, path),
                app_env.resources_path.clone(),
                exec_path,
                app_env.start_args.clone()
            );
            match serde_json::to_string(&process_info) {
                Ok(json) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("GetProcessInfo json serialization error: {}", e).into_bytes(), responder);
                }
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
        NodeCommand::FSAccess { path, mode } => {
            if mode & 1 !=0 && !Path::new(&path).exists() {
                responder.respond(Response::builder().header(ACCESS_CONTROL_ALLOW_ORIGIN, "*").header(CONTENT_TYPE, CONTENT_TYPE_TEXT).body(Vec::from("NOK".to_string().as_bytes())).unwrap());
                return;
            }
            match fs::metadata(path.as_str()) {
                Ok (meta) => {
                    if mode & 4 !=0 && meta.permissions().readonly() {
                        responder.respond(Response::builder().header(ACCESS_CONTROL_ALLOW_ORIGIN, "*").header(CONTENT_TYPE, CONTENT_TYPE_TEXT).body(Vec::from("NOK".to_string().as_bytes())).unwrap());
                        return;
                    }
                    respond_ok(responder);
                },
                Err (e) => {
                    error!("FSAccess error: {}", e);
                    respond_404(responder);
                }
            }
        },
        NodeCommand::FSLstat { path} => {
            let p = Path::new(&path);
            if !p.exists() {
                respond_404(responder);
            } else {
                let mut created:Option<SystemTime>=None;
                let mut modified:Option<SystemTime>=None;
                if let Ok(meta) = p.metadata() {
                    if let Ok(c) = meta.created() {
                        created = Some(c);
                    }
                    if let Ok(m) = meta.modified() {
                        modified = Some(m);
                    }
                }
                let stat = FSStat::new(p.is_dir(), created, modified);
                match serde_json::to_string(&stat) {
                    Ok(json) => {
                        respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                    },
                    Err(e) => {
                        respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSLstat json serialization error: {}", e).into_bytes(), responder);
                    }
                }
            }
        },
        NodeCommand::FSMkdir { path, options } => {
            if Path::new(&path).exists() {
                trace!("FSMkdir path exists {}", path);
                respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), path.into_bytes(), responder);
                return;
            }
            if let Some(options) = options {
                if let Some (recursive) = options.recursive {
                    if recursive {
                        match fs::create_dir_all(path.as_str()) {
                            Ok (_) => {
                                respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), path.into_bytes(), responder);
                                return;
                            },
                            Err (e) => {
                                error!("FSMkdir create_dir_all error: {} {}", path.as_str(), e);
                                respond_404(responder);
                                return;
                            }
                        }
                    }
                }
            }
            match fs::create_dir(path.as_str()) {
                Ok (_) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), path.into_bytes(), responder);
                return;
                },
                Err (e) => {
                    error!("FSMkdir create_dir error: {} {}", path.as_str(), e);
                    respond_404(responder);
                return;
                }
            }
        },
        NodeCommand::FSReadDir { path, options } => {
            let mut recursive = false;
            if let Some(options) = options {
                if let Some(rec) = options.recursive {
                    recursive=rec;
                }
            }
            let mut entries:Vec<FSDirent> = Vec::new();
            fn read_dir(path:String, entries:&mut Vec<FSDirent>, recursive:bool) -> Option<std::io::Error> {
                match fs::read_dir(path) {
                    Ok(rd) => {
                        for e in rd {
                            if let Ok(e) = e {
                                let path = e.path().as_os_str().to_str().unwrap().to_string();
                                if recursive && e.path().is_dir() {
                                    if let Some(error) = read_dir(path.clone(), entries, recursive) {
                                        return Some(error);
                                    }
                                }
                                entries.push(FSDirent::new(path, e.path().is_dir()));
                            }
                        }
                        return None;
                    },
                    Err(e) => {
                        error!("FSReadDir error {}", e);
                        return Some(e);
                    }
                }
            }
            if let Some(error) = read_dir(path, &mut entries, recursive) {
                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSReadDir error: {}", error).into_bytes(), responder);
                return;
            }
            match serde_json::to_string(&entries) {
                Ok(json) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSReadDir json serialization error: {}", e).into_bytes(), responder);
                }
            }
        },
        NodeCommand::FSReadFile { path, options } => {
            match fs::read(path.as_str()) {
                Ok (contents) => {
                    respond_status(StatusCode::OK, mime_guess::from_path(path).first_or_octet_stream().to_string(), contents, responder);
                },
                Err (e) => {
                    error!("FSReadFile error: {} - {}", path, e);
                    respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSReadFile error: {}", e).into_bytes(), responder);
                }
            }
        },
        NodeCommand::FSWriteFile { path, options } => {
            if let Some(data) = data_blob {
                if let Some(options) = options {
                    if let Some(_encoding) = options.encoding {
                        match fs::write(path.as_str(), data) {
                            Ok(_) => {
                                respond_ok(responder);
                            },
                            Err (e) => {
                                error!("FSWriteFile error: {}", e);
                                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSWriteFile error: {}", e).into_bytes(), responder);
                            }
                        }
                        return;
                    }
                }
                match fs::write(path.as_str(), data) {
                    Ok(_) => {
                        respond_ok(responder);
                    },
                    Err (e) => {
                        error!("FSWriteFile error: {}", e);
                        respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSWriteFile error: {}", e).into_bytes(), responder);
                    }
                }
            } else {
                error!("FSWrite error, no data");
                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSWriteFile error, no data").into_bytes(), responder);
            }
        },
        NodeCommand::FSOpen {fd, path, flags, mode } => {
            let write = flags.contains("w") || flags.contains("a");
            match OpenOptions::new().read(true).write(write).create(write).truncate(flags.contains("w")).open(path) {
                Ok(mut file) => {
                    if flags.contains("a") {
                        let _ = file.seek(SeekFrom::End(0));
                    }
                    backend.fs_open(fd, file);
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), fd.to_string().into_bytes(), responder); 
                },
                Err(e) => {
                    respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSOpen error: {}", e).into_bytes(), responder);
                }
            }
        },
        NodeCommand::FSClose { fd } => {
            backend.fs_close(fd);
            respond_ok(responder);
        },
        NodeCommand::FSRead { fd, offset, length, position } => {
            trace!("FSRead {}, {}, {}, {:?}", fd, offset, length, position);
            if let Some(mut file) = backend.fs_get(fd) {
                if let Some(position) = position {
                    let _ = file.seek(SeekFrom::Start(position));
                }
                let _ = file.seek(SeekFrom::Current(offset));
                let mut buf = vec![0; length];
                match file.read(&mut buf) {
                    Ok(read) => {
                        respond_status(StatusCode::OK, CONTENT_TYPE_BIN.to_string(), buf[0..read].to_vec(), responder);
                    },
                    Err(e) => {
                        error!("FSRead error: {}", e);
                        respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSRead error: {}", e).into_bytes(), responder);
                    }
                }
            } else {
                respond_404(responder);
            }
        },
        NodeCommand::FSWrite { fd, offset, length, position } => {
            trace!("FSWrite {}, {}, {}, {:?}", fd, offset, length, position);
            if let Some(mut file) = backend.fs_get(fd) {
                if let Some(position) = position {
                    let _ = file.seek(SeekFrom::Start(position));
                }
                let _ = file.seek(SeekFrom::Current(offset));
                if let Some(mut data) = data_blob {
                    match file.write(&mut data) {
                        Ok(written) => {
                            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), written.to_string().as_bytes().to_vec(), responder);
                        },
                        Err(e) => {
                            error!("FSWrite error: {}", e);
                            respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSWrite error: {}", e).into_bytes(), responder);
                        }
                    }
                } else {
                    error!("FSWrite error, no data");
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSWrite error, no data").into_bytes(), responder);
                }
                
            } else {
                respond_404(responder);
            }
        },
        NodeCommand::FSRealPath { path } => {
            let rp = Path::new(path.as_str()).as_os_str().to_str().unwrap().to_string();
            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), rp.as_bytes().to_vec(), responder);
        },
        NodeCommand::FSFdatasync { fd } => {
            if let Some(file) = backend.fs_get(fd) {
                let _ = file.sync_all();
                respond_ok(responder);
            } else {
                respond_404(responder);
            }
        },
        NodeCommand::FSUnlink { path } => {
            let p = Path::new(path.as_str());
            if p.is_file() {
                let _ = fs::remove_file(path);
            } else if p.is_symlink() {
                let _ = symlink::remove_symlink_file(path);
            }
            respond_ok(responder);
        },
        NodeCommand::FSRename { old_path, new_path } => {
            let _ = fs::rename(old_path, new_path);
            respond_ok(responder);
        },
        NodeCommand::HTTPRequest { options } => {
            tokio_runtime.spawn(
                async move {
                    let url = "https://".to_string()+options.hostname.as_str()+":"+options.port.to_string().as_str()+options.path.as_str();
                    
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36"));
                    headers.insert(reqwest::header::ACCEPT, reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
                    if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).default_headers(headers).build() {
                        let method:Method;
                        match Method::try_from(options.method.as_str()) {
                            Ok(m) => {
                                method=m;
                            },
                            Err(_e) => {
                                respond_client_error(format!("invalid method {}", options.method), responder);
                                return;
                            }
                        }
                        let rurl:Url;
                        match Url::parse(url.as_str()) {
                            Ok(r) => {
                                rurl=r;
                            },
                            Err(_e) => {
                                respond_client_error(format!("invalid url {}", url), responder);
                                return;
                            }
                        }

                        match client.execute(Request::new(method, rurl)).await {
                            Ok(response) => {
                                let headers = response.headers().clone();
                                match response.bytes().await {
                                    Ok(body) => {
                                        let mut rbuilder = Response::builder()
                                            .status(StatusCode::OK)
                                            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*");
                                        for h in headers {
                                            if let Some(hname) = h.0 {
                                                rbuilder = rbuilder.header(hname, h.1);
                                            }
                                        }
                                        responder.respond(rbuilder.body(Vec::from(body)).unwrap());
                                    }, 
                                    Err(e) => {
                                        respond_client_error(format!("could not read response {}", e), responder);
                                    }
                                }
                            },
                            Err(e) => {
                                respond_client_error(format!("could not send request {}", e), responder);
                            }
                        }
                    }
                }
            );
        },
        NodeCommand::ChildProcessSpawn { cmd, args } => {
            child_process_spawn(cmd, args, backend, tokio_runtime, proxy, command_sender, responder);
        },
        NodeCommand::ChildProcessStdinWrite { pid } => {
            backend.child_process_callback(pid, "stdin".to_string(), data_blob);
            respond_ok(responder);
        },
        NodeCommand::ChildProcessDisconnect { pid } => {
            backend.child_process_disconnect(pid);
            respond_ok(responder);
        },
        NodeCommand::FSWatch { path, wid, options } => {
            let mut mode = RecursiveMode::NonRecursive;
            if let Some(options) = options {
                if let Some(rec) = options.recursive {
                    if rec {
                        mode=RecursiveMode::Recursive;
                    }
                }
            }
            let w_proxy = proxy.clone();
            let w_command_sender = command_sender.clone();
            let w_wid = wid.clone();
            match RecommendedWatcher::new(
                move |res| {
                    if let Ok(event) = res {
                        trace!("fswatch receive event {:?}", event);
                        let _ = send_command(&w_proxy, &w_command_sender, BackendCommand::FSWatchEvent { wid: w_wid.clone(), event: event });
                    }
                },
                Config::default()
            ) {
                Ok(mut watcher) => {
                    respond_ok(responder);
                    let _ = watcher.watch(path.as_ref(), mode);
                    backend.watch_start(wid, watcher);                  
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).to_string().into_bytes(), responder); 
                }
            }
        },
        NodeCommand::FSWatchClose { wid } => {
            backend.watch_stop(wid);
            respond_ok(responder);
        },
        NodeCommand::NETCreateServer {hook, options } => {
            trace!("NETCreateServer {}", hook);
            ipc_server(hook, tokio_runtime, proxy, command_sender, responder);
        },
        NodeCommand::NETCloseServer { id } => {
            backend.net_server_close(id);
            respond_ok(responder);
        },
        NodeCommand::NETCloseConnection { id } => {
            backend.net_connection_close(id);
            respond_ok(responder);
        },
        NodeCommand::NETCreateConnection { hook, id } => {
            trace!("NETCreateConnection {}, {}", hook, id);
            ipc_connection(hook, id, tokio_runtime, proxy, command_sender, responder);
        },
        NodeCommand::NETWriteConnection { id } => {
            trace!("NETWriteConnection {}", id);
            if let Some(data) = data_blob {
                backend.net_write_connection(id, data);
                respond_ok(responder);
            } else {
                error!("NETWriteConnection error, no data");
                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("NETWriteConnection error, no data").into_bytes(), responder);
            }
        },
        NodeCommand::NETSetTimeout { id, timeout } => {
            trace!("NETSetTimeout {}, {}", id, timeout);
            backend.net_set_timeout(id, timeout);
            respond_ok(responder);
        }
        NodeCommand::GetDataBlob { id } => {
            if let Some(data) = backend.get_data_blob(id) {
                respond_status(StatusCode::OK, CONTENT_TYPE_BIN.to_string(), data, responder);
            } else {
                respond_404(responder);
            }
        },
        NodeCommand::Addon { data } => {
            process_command(tokio_runtime, app_env, proxy, backend, data, responder, data_blob); 
        },
    }
}