use std::{collections::HashMap, fs::{self, File}, path::{Path, PathBuf}, thread::{self}};

use include_dir::{include_dir, Dir};
use reqwest::StatusCode;
use urlencoding::decode;
use log::{info, trace, error};
use std::io::Read;

use crate::{ipcchannel::IPCResponse, types::Responder};

pub const CONTENT_TYPE_TEXT: &str = "text/plain;charset=utf-8";
pub const CONTENT_TYPE_HTML: &str = "text/html;charset=utf-8";
pub const CONTENT_TYPE_CSS: &str = "text/css;charset=utf-8";
pub const CONTENT_TYPE_JSON: &str = "application/json;charset=utf-8";
pub const CONTENT_TYPE_JS: &str = "text/javascript;charset=utf-8";
pub const CONTENT_TYPE_BIN: &str = "application/octet-stream";
pub const JS_DIR_FRONTEND: Dir = include_dir!("src/js/frontend");

pub fn append_js_scripts(script:String, dir:Dir, filter:Option<&str>) -> String {
    let mut res = script.clone();
    for f in dir.files() {
        let path = f.path().file_name().unwrap().to_str().unwrap().to_string();
        if let Some(filter) = filter {
            if path.ends_with(filter) {
                res += f.contents_utf8().unwrap_or("");
            }
        }
    }
    res
}

pub fn build_file_map(dir:&Dir) -> HashMap<String, Vec<u8>> {
    let mut res:HashMap<String, Vec<u8>> = HashMap::new();
    for f in dir.files() {
        let path = f.path().to_str().unwrap().to_string();
        let content = f.contents();
        res.insert(path, Vec::from(content));
    }
    for d in dir.dirs() {
        let d_res = build_file_map(d);
        res.extend(d_res.into_iter())
    }
    res
}

fn respond_not_found(module:bool, responder:Responder) {
    if module {
        respond_status(StatusCode::MOVED_PERMANENTLY, CONTENT_TYPE_HTML.to_string(), "package".to_string().into_bytes(), responder);
    } else {
        respond_status(StatusCode::NOT_FOUND, CONTENT_TYPE_HTML.to_string(), "not found".to_string().into_bytes(), responder);
    }
}

pub fn parse_http_url_path(path:&str) -> Option<(String, String, String, String)> {
    let pparts:Vec<&str> = path.split("/").collect();
    if let Some(host) = pparts.get(1) {
        let hparts:Vec<&str> = host.split("@").collect();
        if let Some(http_id_str) =  hparts.get(0) {
            let http_id = http_id_str.to_string();
            if let Some(protocol) =  hparts.get(1) {
                if let Some(url_root) = pparts.get(2) {
                    let url = format!("/{}", pparts[3..pparts.len()].join("/"));
                    return Some((http_id, protocol.to_string(), url_root.to_string(), url));
                }
            }
        }
    }
    None
}

pub fn handle_file_request(module:bool, path:String, full_path:PathBuf, resources:&HashMap<String, Vec<u8>>, responder:Responder)  {
    let resources_rt=resources.clone();
    
            if let Some(res) = resources_rt.get(&path) {
                respond_status(
                    StatusCode::OK, 
                    mime_guess::from_path(path).first_or_octet_stream().to_string(), 
                    res.to_vec(), responder); 
            } else if full_path.exists() {
                match File::open(full_path.clone()) {
                    Ok (mut f) => {
                        let mut buffer = Vec::new();
                        match f.read_to_end(&mut buffer) {
                            Ok(_r) => {
                                respond_status(
                                    StatusCode::OK, 
                                    mime_guess::from_path(full_path).first_or_octet_stream().to_string(), 
                                    buffer, responder);
                            },
                            Err(_e) => {
                                trace!("file not found {}", full_path.to_str().unwrap());
                                respond_not_found(module, responder);
                            }
                        }
                        
                    },
                    Err (_e) => {
                        trace!("file not found {}", full_path.as_os_str().to_str().unwrap());
                        respond_not_found(module, responder);
                    }
                }
            } else {
                trace!("file not found {}", path);
                respond_not_found(module, responder);
            }
       
}

pub fn read_file(path:&String) -> Option<Vec<u8>> {
    match File::open(path.clone()) {
        Ok (mut f) => {
            let mut buffer = Vec::new();
            match f.read_to_end(&mut buffer) {
                Ok(_r) => {
                   return Some(buffer);
                },
                Err(e) => {
                    error!("file read error {} {}", path, e);
                    return None;
                }
            }
        },
        Err (_e) => {
            error!("file not found {}", path);
            return None;
        }
    }
}

pub fn get_message_data_http(query:Option<&str>, request: Vec<u8>) -> Option<(String, Option<Vec<u8>>)> {
    let cmdmsg:String;
    let data_blob:Option<Vec<u8>>;
    if let Some(queryenc) = query {
        if let Ok(query) = decode(queryenc) {
            cmdmsg=query.to_string();
            data_blob = Some(request);
        } else {
            error!("url decoder error");
            return None;
        }
    } else {
        let msgr =  String::from_utf8(request);
        match msgr {
            Ok(msg) => {
                trace!("backend cmd request body {}", msg.as_str());
                cmdmsg=msg;
                data_blob=None;
            },
            Err(e) => {
                error!("utf8 error {}", e);
                return None;
            }
        }
    }
    return Some((cmdmsg, data_blob));
}

pub fn escape(s:&String) -> String {
    s.replace("\\", "\\\\").replace("'", "\\'").replace("\n", "\\n").replace("\r", "\\r")
}

pub fn escapemsg(s:&String) -> String {
    s.replace("\\", "\\\\").replace("\"", "\\\"")
}

fn respond_http(sender: tokio::sync::mpsc::Sender<IPCResponse>, body:Vec<u8>, content_type: String, status:StatusCode) {
    thread::spawn(move || {
        let _ = sender.blocking_send(IPCResponse::new(body, content_type, status));
    });
}

pub fn respond_ok(responder:Responder) {
    let body = Vec::from("OK".to_string().as_bytes());
    match responder {
        Responder::HttpProtocol { sender } => {
            respond_http(sender, body, CONTENT_TYPE_HTML.to_string(), StatusCode::OK);
        },
        _=>()
    }
}

pub fn respond_404(responder:Responder) {
    let body = Vec::from("404".to_string().as_bytes());
    match responder {
        Responder::HttpProtocol { sender } => {
            respond_http(sender, body, CONTENT_TYPE_HTML.to_string(), StatusCode::NOT_FOUND);
        },
        _=>()
    }
}

pub fn respond_status(status:StatusCode, content_type: String, body:Vec<u8>, responder:Responder) {
    match responder {
       Responder::HttpProtocol { sender } => {
            respond_http(sender, body, content_type, status);
        },
        _=>()
    }
}

pub fn respond_client_error(error:String, responder:Responder) {
    match responder {
        Responder::HttpProtocol { sender } => {
            respond_http(sender, Vec::from(error.as_bytes()), CONTENT_TYPE_TEXT.to_string(), StatusCode::BAD_REQUEST);
        },
        _=>()
    }
}

pub fn check_and_create_dir(dir:&Path) {
    if !dir.exists() {
        info!("creating directory: {}", dir.as_os_str().to_str().unwrap());
        let _ = fs::create_dir_all(dir);
    }
}
pub fn is_module_request(host:Option<&str>) -> bool {
    if let Some(host) = host {
        if host=="electrico-mod" {
            return true;
        }
    }
    false
}
