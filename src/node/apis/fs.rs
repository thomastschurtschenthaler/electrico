use std::{ffi::OsStr, fs::{self, remove_dir, remove_file, OpenOptions}, io::{Read, Seek, SeekFrom, Write}, path::Path, time::SystemTime};

use log::{error, debug, trace};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use reqwest::{header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE}, StatusCode};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::{http::Response, RequestAsyncResponder};

use crate::{backend::Backend, common::{respond_404, respond_ok, respond_status, CONTENT_TYPE_BIN, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, node::{apis::types::FSDirent, common::send_command, node::AppEnv}, types::{BackendCommand, ElectricoEvents}};

use super::types::{FSCommand, FSStat};

pub fn process_fs_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:FSCommand,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    
    let command_sender = backend.command_sender();
    match command {
        FSCommand::Access { path, mode } => {
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
        FSCommand::Lstat { path} => {
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
        FSCommand::Rm { path, options } => {
            debug!("NodeCommand::FSRm path {}", path);
            let p = Path::new(&path);
            if p.is_file() {
                let _ = remove_file(p);
            } else if p.is_dir() {
                let _ = remove_dir(p);
            }
            respond_ok(responder);
        },
        FSCommand::Mkdir { path, options } => {
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
        FSCommand::ReadDir { path, options } => {
            let mut recursive = false;
            if let Some(options) = options {
                if let Some(rec) = options.recursive {
                    recursive=rec;
                }
            }
            let mut entries:Vec<FSDirent> = Vec::new();
            fn read_dir(path:String, entries:&mut Vec<FSDirent>, recursive:bool) -> Option<std::io::Error> {
                match fs::read_dir(&path) {
                    Ok(rd) => {
                        for e in rd {
                            if let Ok(e) = e {
                                let dpath = e.path().as_os_str().to_str().unwrap().to_string();
                                let name = e.path().file_name().unwrap_or(OsStr::new("")).to_str().unwrap_or("").to_string();
                                if recursive && e.path().is_dir() {
                                    if let Some(error) = read_dir(dpath, entries, recursive) {
                                        return Some(error);
                                    }
                                }
                                entries.push(FSDirent::new(path.clone(), name, e.path().is_dir()));
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
        FSCommand::ReadFile { path, options } => {
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
        FSCommand::WriteFile { path, options } => {
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
        FSCommand::Open {fd, path, flags, mode } => {
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
        FSCommand::Close { fd } => {
            backend.fs_close(fd);
            respond_ok(responder);
        },
        FSCommand::Read { fd, offset, length, position } => {
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
        FSCommand::Write { fd, offset, length, position } => {
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
        FSCommand::RealPath { path } => {
            let rp = Path::new(path.as_str()).as_os_str().to_str().unwrap().to_string();
            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), rp.as_bytes().to_vec(), responder);
        },
        FSCommand::Fdatasync { fd } => {
            if let Some(file) = backend.fs_get(fd) {
                let _ = file.sync_all();
                respond_ok(responder);
            } else {
                respond_404(responder);
            }
        },
        FSCommand::Unlink { path } => {
            let p = Path::new(path.as_str());
            if p.is_file() {
                let _ = fs::remove_file(path);
            } else if p.is_symlink() {
                let _ = symlink::remove_symlink_file(path);
            }
            respond_ok(responder);
        },
        FSCommand::Rename { old_path, new_path } => {
            let _ = fs::rename(old_path, new_path);
            respond_ok(responder);
        },
        FSCommand::Watch { path, wid, options } => {
            let mut mode = RecursiveMode::NonRecursive;
            if let Some(options) = options {
                if let Some(rec) = options.recursive {
                    if rec {
                        mode=RecursiveMode::Recursive;
                    }
                }
            }
            let w_wid = wid.clone();
            match RecommendedWatcher::new(
                move |res| {
                    if let Ok(event) = res {
                        trace!("fswatch receive event {:?}", event);
                        let w_proxy = proxy.clone();
                        let w_command_sender = command_sender.clone();
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
        FSCommand::WatchClose { wid } => {
            backend.watch_stop(wid);
            respond_ok(responder);
        },
    }
}