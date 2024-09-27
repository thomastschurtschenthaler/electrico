use std::{fs, io::{Read, Write}, path::Path, process::{Command, Stdio}, sync::mpsc::{self, channel, Receiver, Sender}};
use base64::prelude::*;
use log::{debug, error, info, trace, warn};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use reqwest::{header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE}, Method, Request, StatusCode, Url};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::{http::Response, webview_version, RequestAsyncResponder};

use crate::{common::{respond_404, respond_client_error, respond_ok, respond_status, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, node::types::{Process, ProcessEnv, ProcessVersions}, types::{BackendCommand, ElectricoEvents}};
use super::types::{ConsoleLogLevel, FSStat, NodeCommand};

pub struct AppEnv {
    pub start_args: Vec<String>,
    pub app_name:Option<String>,
    pub resources_path:String
}

impl AppEnv {
    pub fn new(resources_path:String) -> AppEnv {
        let mut args = Vec::new();
        for arg in std::env::args() {
            args.push(arg);
        }
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

fn send_command(proxy:&EventLoopProxy<ElectricoEvents>, command_sender:&Sender<BackendCommand>, command:BackendCommand) {
    let _ = command_sender.send(command);
    let _ = proxy.send_event(ElectricoEvents::Noop);
}

pub fn process_node_command(tokio_runtime:&Runtime, app_env:&AppEnv,
        proxy:EventLoopProxy<ElectricoEvents>,
        command_sender:Sender<BackendCommand>,
        command:NodeCommand,
        responder:RequestAsyncResponder)  {
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

            let process_info = Process::new(platform.to_string(), 
                ProcessVersions::new(node, chrome, electron), 
                ProcessEnv::new(node_env, electron_is_dev, home),
                app_env.resources_path.clone());
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
                let stat = FSStat::new(p.is_dir());
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
        NodeCommand::FSWriteFile { path, data, options } => {
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
            match BASE64_STANDARD.decode(data) {
                Ok(decoded) => {
                    match fs::write(path.as_str(), decoded) {
                        Ok(_) => {
                            respond_ok(responder);
                        },
                        Err (e) => {
                            error!("FSWriteFile error: {}", e);
                            respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("FSWriteFile error: {}", e).into_bytes(), responder);
                        }
                    }
                },
                Err(e) => {
                    error!("FSWriteFile base64 decode error: {}", e);
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("FSWriteFile base64 decode error: {}", e).into_bytes(), responder);
                }
            }
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
            let mut pargs:Vec<String> = Vec::new();
            if let Some(args) = args {
                pargs = args;
            }
            match Command::new(cmd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .args(pargs).spawn() {
                Ok(mut child) => {
                    let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
                    let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessStart { pid: child.id().to_string(), sender: sender });
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), child.id().to_string().into_bytes(), responder); 
                    tokio_runtime.spawn(
                        async move {
                            let mut stdout;
                            let mut stderr;
                            let mut stdin;
                            match child.stdout.take() {
                                Some(chstdout) => {
                                    stdout=chstdout;
                                }, 
                                None => {
                                    error!("ChildProcessSpawn stdout not available");
                                    let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:None});
                                    return;
                                }
                            }
                            match child.stderr.take() {
                                Some(chstderr) => {
                                    stderr=chstderr;
                                }, 
                                None => {
                                    error!("ChildProcessSpawn stderr not available");
                                    let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:None});
                                    return;
                                }
                            }
                            match child.stdin.take() {
                                Some(chstdin) => {
                                    stdin=chstdin;
                                }, 
                                None => {
                                    error!("ChildProcessSpawn stdid not available");
                                    let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:None});
                                    return;
                                }
                            }
                            let mut exit_code:Option<i32> = None;
                            loop {
                                if let Ok(data) = receiver.try_recv() {
                                    trace!("writing stdin {}", data.len());
                                    let _ = stdin.write(data.as_slice());
                                }
                                let mut stdinread:usize=0;
                                let mut stderrread:usize=0;
                                let stdout_buf:&mut [u8] = &mut [0; 1024];
                                if let Ok(read) = stdout.read(stdout_buf) {
                                    trace!("stdout read {}", read);
                                    stdinread = read;
                                    if read>0 {
                                        let data:Vec<u8> = stdout_buf[0..read].to_vec();
                                        let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessCallback { pid:child.id().to_string(), stream:"stdout".to_string(), data:data });
                                    }
                                }
                                let stderr_buf:&mut [u8] = &mut [0; 1024];
                                if let Ok(read) = stderr.read(stderr_buf) {
                                    trace!("stderr read {}", read);
                                    stderrread = read;
                                    if read>0 {
                                        let data:Vec<u8> = Vec::from(stderr_buf);
                                        let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessCallback { pid:child.id().to_string(), stream:"stderr".to_string(), data:data });
                                    }
                                }
                                match child.try_wait() {
                                    Ok(event) => {
                                        if let Some(event) = event {
                                            exit_code = event.code();
                                        }
                                    }
                                    Err(e) => {
                                        error!("ChildProcessSpawn try_wait error: {}", e);
                                        break;
                                    }
                                }
                                if let Some(_exit_code) = exit_code {
                                    if stdinread==0 && stderrread==0 {
                                        break;
                                    }
                                }
                            }
                            let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:exit_code});
                        }
                    );         
                },
                Err(e) => {
                    respond_client_error(format!("Error: {}", e), responder);
                }
            }
        },
        NodeCommand::ChildProcessStdinWrite { pid, data } => {
            let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessCallback { pid:pid, stream:"stdin".to_string(), data:data.into_bytes() });
            respond_ok(responder);
        },
        NodeCommand::FSWatch { path, wid, options } => {
            tokio_runtime.spawn(
                async move {
                    let (tx, rx): (Sender<Result<Event, notify::Error>>, Receiver<Result<Event, notify::Error>>) = channel();
                    match RecommendedWatcher::new(
                        move |res| {
                            let _ = tx.send(res);
                        },
                        Config::default()
                    ) {
                        Ok(mut watcher) => {
                            respond_ok(responder);
                            let _ = watcher.watch(path.as_ref(), RecursiveMode::Recursive);
                            let (sender, receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();
                            let _ = send_command(&proxy, &command_sender, BackendCommand::FSWatchStart { wid: wid.clone(), sender: sender });
                            loop {
                                if let Ok(res) = rx.try_recv() {
                                    if let Ok(event) = res {
                                        trace!("fswatch receive event {:?}", event);
                                        let _ = send_command(&proxy, &command_sender, BackendCommand::FSWatchEvent { wid: wid.clone(), event: event });
                                    }
                                }
                                if let Ok(_stop) = receiver.try_recv() {
                                    trace!("fswatch receive stop");
                                    break;
                                }
                            }
                        },
                        Err(e) => {
                            respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).to_string().into_bytes(), responder); 
                        }
                    }
                }
            );
        },
        NodeCommand::FSWatchClose { wid } => {
            let _ = send_command(&proxy, &command_sender, BackendCommand::FSWatchStop { wid });
            respond_ok(responder);
        }
    }
}