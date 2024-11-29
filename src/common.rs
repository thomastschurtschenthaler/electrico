use std::{collections::HashMap, fs::{self, File}, path::{Path, PathBuf}};

use include_dir::{include_dir, Dir};
use queues::{IsQueue, Queue};
use reqwest::{header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE}, StatusCode};
use tokio::runtime::Runtime;
use urlencoding::decode;
use wry::{http::{Request, Response}, RequestAsyncResponder};
use log::{debug, info, trace, error};
use std::io::Read;

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

fn respond_not_found(module:bool, responder:RequestAsyncResponder) {
    if module {
        respond_status(StatusCode::MOVED_PERMANENTLY, CONTENT_TYPE_HTML.to_string(), "package".to_string().into_bytes(), responder);
    } else {
        respond_status(StatusCode::NOT_FOUND, CONTENT_TYPE_HTML.to_string(), "not found".to_string().into_bytes(), responder);
    }
}

pub fn handle_file_request(tokio_runtime:&Runtime, module:bool, path:String, full_path:PathBuf, resources:&HashMap<String, Vec<u8>>, responder:RequestAsyncResponder)  {
    let resources_rt=resources.clone();
    tokio_runtime.spawn(
        async move {
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
    );
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

pub fn get_message_data(request: &Request<Vec<u8>>) -> Option<(String, Option<Vec<u8>>)> {
    let cmdmsg:String;
    let data_blob:Option<Vec<u8>>;
    if let Some(queryenc) = request.uri().query() {
        if let Ok(query) = decode(queryenc) {
            cmdmsg=query.to_string();
            data_blob = Some(request.body().to_vec());
        } else {
            error!("url decoder error");
            return None;
        }
    } else {
        let msgr =  String::from_utf8(request.body().to_vec());
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

pub fn respond_ok(responder:RequestAsyncResponder) {
    responder.respond(Response::builder()
        .status(StatusCode::OK)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(CONTENT_TYPE, CONTENT_TYPE_HTML)
        .body(Vec::from("OK".to_string().as_bytes())).unwrap())
}

pub fn respond_404(responder:RequestAsyncResponder) {
    responder.respond(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(CONTENT_TYPE, CONTENT_TYPE_HTML)
        .body(Vec::from("404".to_string().as_bytes())).unwrap())
}

pub fn respond_status(status:StatusCode, content_type: String, body:Vec<u8>, responder:RequestAsyncResponder) {
    responder.respond(Response::builder()
        .status(status)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(CONTENT_TYPE, content_type)
        .body(body).unwrap())
}

pub fn respond_client_error(error:String, responder:RequestAsyncResponder) {
    responder.respond(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(CONTENT_TYPE, CONTENT_TYPE_TEXT)
        .body(Vec::from(error.as_bytes())).unwrap())
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
pub struct DataQueue {
    data_blobs:HashMap<String, Queue<Vec<u8>>>
}
impl DataQueue {
    pub fn new() -> DataQueue {
        DataQueue {
            data_blobs:HashMap::new()
        }
    }
    pub fn add(&mut self, k:&String, data:Vec<u8>) {
        if let Some(q) = self.data_blobs.get_mut(k) {
           let _ = q.add(data);
        } else {
            let mut q = Queue::new();
            let _ = q.add(data);
            self.data_blobs.insert(k.clone(), q);
        }
    }
    pub fn take(&mut self, k:&String) -> Option<Vec<u8>>{
        if let Some(q) = self.data_blobs.get_mut(k) {
            let ret:Option<Vec<u8>>;
            if let Ok(data) = q.peek() {
                let _ = q.remove();
                ret = Some(data);
            } else {
                ret = None;
            }
            if q.size()==0 {
                self.data_blobs.remove(k);
            }
            return ret;
        }
        None
    }
    pub fn size(&mut self, k:&String) -> usize {
        if let Some(q) = self.data_blobs.get_mut(k) {
           return q.size();
        }
        return 0;
    }
}
