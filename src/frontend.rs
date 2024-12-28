use std::net::{IpAddr, SocketAddr};
use std::thread;
use std::time::Duration;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use log::{debug, error, info, trace, warn};
use include_dir::{include_dir, Dir};
use reqwest::StatusCode;
use substring::Substring;
use std::sync::mpsc::{self, Receiver, Sender};
use uuid::Uuid;
use std::{collections::HashMap, fs, path::PathBuf};
use tao::{dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize}, event_loop::{EventLoopProxy, EventLoopWindowTarget}, window::{Icon, Window, WindowBuilder, WindowId}};
use serde_json::Error;
use std::net::ToSocketAddrs;
use wry::{WebView, WebViewBuilder};
use crate::common::get_message_data_http;
use crate::ipcchannel::IPCResponse;
use crate::{common::{append_js_scripts, escape, DataQueue, CONTENT_TYPE_TEXT, JS_DIR_FRONTEND}, electron::types::{BrowserWindowCreateParam, Rectangle}, types::{Command, ElectricoEvents, FrontendCommand}};

pub struct FrontendWindow {
    window:Window,
    webview:WebView,
    id:WindowId,
    http_id: String,
    client_path_base:Option<PathBuf>,
}
impl FrontendWindow {
    pub fn new(window:Window, webview:WebView, id:WindowId, http_id:String) -> FrontendWindow {
        FrontendWindow {
            window:window,
            webview:webview,
            id:id,
            http_id,
            client_path_base:None
        }
    }
    pub fn set_client_path_base(&mut self, client_path_base:Option<PathBuf>) {
        self.client_path_base = client_path_base;
    }
}

fn http_port(port:&Option<u16>) ->u16 {
    let http_port:u16;
    if let Some(port) = port {
        http_port = port.clone();
    } else {
        panic!("no http server");
    }
    return http_port;
}

pub struct Frontend {
    window_ids:HashMap<WindowId, String>,
    windows:HashMap<String, FrontendWindow>,
    window_http_ids:HashMap<String, String>,
    opened_windows:usize,
    frontendalljs:String,
    rsrc_dir:PathBuf,
    data_queue:DataQueue,
    http_port_start:u16,
    http_port:Option<u16>,
    file_protocols:Vec<String>
}
impl Frontend {
    pub fn new(rsrc_dir:PathBuf) -> Frontend {
        let mut frontendalljs:String = String::new();
        const JS_DIR_SHARED: Dir = include_dir!("src/js/shared");
        frontendalljs = append_js_scripts(frontendalljs, JS_DIR_SHARED, Some(".js"));
        frontendalljs = append_js_scripts(frontendalljs, JS_DIR_FRONTEND, Some("electrico.js"));
        Frontend {
            window_ids:HashMap::new(),
            windows:HashMap::new(),
            window_http_ids:HashMap::new(),
            opened_windows:0,
            frontendalljs:frontendalljs,
            rsrc_dir:rsrc_dir,
            data_queue:DataQueue::new(),
            http_port_start:3000,
            http_port:None,
            file_protocols:Vec::new()
        }
    }
    
    pub fn create_window(&mut self, id:String, event_loop:&EventLoopWindowTarget<ElectricoEvents>, proxy:EventLoopProxy<ElectricoEvents>, config_params:BrowserWindowCreateParam) {
        fn find_tcp_port(http_port_start:u16) -> u16 {
            for p in http_port_start..http_port_start+1000 {
                let addr = SocketAddr::from(([127, 0, 0, 1], p));
                if let Ok(_) = std::net::TcpListener::bind(addr) {
                    return p;
                }
            }
            panic!("create_tcp_listener - no free port");
        }

        #[tokio::main(flavor = "multi_thread", worker_threads = 30)]
        async fn start_http_server(proxy:EventLoopProxy<ElectricoEvents>, port:u16, protocols:Vec<String>) {    
            trace!("start_http_server");
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            let listener= TcpListener::bind(addr).await.expect("start_http_server - TcpListener failed");
            loop {
                let (stream, _) = listener.accept().await.expect("start_http_server listener.accept() failed");
                let io = TokioIo::new(stream);
                let s_proxy = proxy.clone();
                let protocols = protocols.clone();
                tokio::task::spawn(async move {
                    let protocols = protocols.clone();
                    let service = service_fn(move |request:Request<hyper::body::Incoming>| {
                        let pparts:Vec<&str> = request.uri().path().split("/").collect();
                        trace!("http server - got request: {}, {:?}", request.uri().to_string(), request.uri().authority());
                        let mut urlparts:Option<(String, String, String, String)> = None;
                        if let Some(host) = pparts.get(1) {
                            let hparts:Vec<&str> = host.split("@").collect();
                            if let Some(http_id_str) =  hparts.get(0) {
                                let http_id = http_id_str.to_string();
                                if let Some(protocol) =  hparts.get(1) {
                                    if let Some(url_root) = pparts.get(2) {
                                        let url = format!("/{}", pparts[3..pparts.len()].join("/"));
                                        urlparts = Some((http_id, protocol.to_string(), url_root.to_string(), url));
                                    }
                                }
                            }
                        }
                        let mut query:Option<String> = None;
                        if let Some(q) = request.uri().query() {
                            query = Some(q.to_string());
                        } 
                        let s2_proxy = s_proxy.clone();
                        let (req_sender, req_receiver): (Sender<IPCResponse>, Receiver<IPCResponse>) = mpsc::channel();
                        let protocols = protocols.clone();
                        return async move {
                            if let Some((http_id, protocol, url_root, url)) = urlparts {
                                trace!("http_server processing request:{http_id}, {protocol}, {url_root}, {url}");
                                let url2 = url.clone();
                                if let Ok(body) = request.collect().await {
                                    let data = body.to_bytes().to_vec();
                                    if is_electrico_ipc(&url) {
                                        trace!("http_server - electrico-ipc request:{url}");
                                        handle_electrico_ipc(&http_id, &s2_proxy, url, query.clone(), data, req_sender);
                                    } else {
                                        if url_root == "electrico-mod" || protocol == "electrico-file" {
                                            let module = url_root == "electrico-mod"; 
                                            trace!("browser file protocol request {}; {}", url, module);
                                            let _ = s2_proxy.send_event(ElectricoEvents::ExecuteCommand {command:Command::BrowserWindowReadFile {
                                                http_id, 
                                                file_path: module_file_path(url), 
                                                module
                                            }, responder:crate::types::Responder::HttpProtocol { sender:req_sender }, data_blob:None});
                                        } else {
                                            let mut path = url;
                                            if let Some(query) = query.clone() {
                                                path = format!("{path}?{query}");
                                            }
                                            trace!("custom file protocol request: {}, {}", protocol, path);
                                            let _ = s2_proxy.send_event(ElectricoEvents::ExecuteCommand {command:Command::PostIPC {
                                                http_id,
                                                nonce:None,
                                                request_id:Uuid::new_v4().to_string(),
                                                params: format!("[\"__electrico_protocol\", \"{}\", \"{}\"]", escape(&protocol), path)
                                            }, responder:crate::types::Responder::HttpProtocol { sender:req_sender}, data_blob:None});
                                        }
                                    }
                                    if let Ok(r) = req_receiver.recv_timeout(Duration::from_secs(3)) {
                                        trace!("http - request response: {}", r.params.len());
                                        let mut r_body = r.params; 
                                        if let Some(query)  = query {
                                            if query.ends_with("electrico_hostpage=true") {
                                                let mut host_page_html = String::from_utf8(r_body).expect("host page utf-8 failed");
                                                for p in protocols {
                                                    if let Some(_) = host_page_html.find(format!("{p}:").as_str()) {
                                                        host_page_html = host_page_html.replace(format!("{p}:").as_str(), format!("*.localhost:{port}").as_str());
                                                    }
                                                }
                                                r_body = host_page_html.as_bytes().to_vec();
                                            }
                                        }
                                        return Ok::<_, Error>(Response::builder()
                                            .header("Content-Type", r.mime_type)
                                            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                            .status(r.status)
                                            .body(Full::new(Bytes::from(r_body))).expect("http body full failed"));
                                    } else {
                                        error!("http - request timed out:{url2}");
                                    }
                                }
                            }
                            error!("http - invalid http request");
                            return Ok::<_, Error>(Response::builder().status(StatusCode::BAD_REQUEST).body(Full::new(Bytes::from(StatusCode::BAD_REQUEST.to_string()))).expect("http body full failed"));
                        };
                    });
                    if let Err(err) = http1::Builder::new()
                        .keep_alive(true)
                        .serve_connection(io, service)
                        .await
                    {
                        error!("Error serving connection: {:?}", err);
                    }
                });
            }   
        }
        if self.http_port==None {
            let port = find_tcp_port(self.http_port_start);
            let s_proxy = proxy.clone();
            let protocols = self.file_protocols.clone();
            thread::spawn(move || {
                start_http_server(s_proxy, port, protocols);
            });
            self.http_port=Some(port);
        }

        fn handle_electrico_ipc(http_id: &String, proxy: &EventLoopProxy<ElectricoEvents>, url:String, query:Option<String>, request: Vec<u8>, sender:Sender<IPCResponse>) {
            trace!("frontend ipc request {}", url);
            let message_data:Option<(String, Option<Vec<u8>>)> = get_message_data_http(query, request);
            if let Some(message_data) = message_data {
                trace!("frontend ipc request action {}", message_data.0.as_str());
                let commandr:Result<FrontendCommand, Error> = serde_json::from_str(message_data.0.as_str());
                match commandr {
                    Ok (command) => {
                        match command {
                            FrontendCommand::PostIPC {request_id, nonce, params } => {
                                trace!("frontend ipc call {} {}", nonce, params);
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::PostIPC {http_id:http_id.clone(), nonce:Some(nonce), request_id, params}, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:message_data.1});
                            },
                            FrontendCommand::GetProcessInfo {nonce} => {
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::FrontendGetProcessInfo {http_id:http_id.clone(), nonce:nonce}, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:None});
                            },
                            FrontendCommand::DOMContentLoaded {title } => {
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::DOMContentLoaded {http_id:http_id.clone(), title}, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:None});
                            },
                            FrontendCommand::Alert {message } => {
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::Electron { invoke: crate::electron::types::ElectronCommand::Api { data: format!("{{\"api\":\"Dialog\", \"command\":{{\"action\":\"ShowMessageBoxSync\", \"options\":{{\"message\":\"{}\"}}}}}}", escape(&message)) } }, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:None});
                            },
                            FrontendCommand::GetDataBlob { id } => {
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::FrontendGetDataBlob { id }, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:None});
                            },
                            FrontendCommand::GetProtocols => {
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::FrontendGetProtocols, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:None});
                            }
                        }
                    }
                    Err(e) => {
                        error!("json serialize error {}", e.to_string());
                    }
                }
            } else {
                let _ = sender.send(IPCResponse::new("data error".to_string().as_bytes().to_vec(), CONTENT_TYPE_TEXT.to_string(), StatusCode::BAD_GATEWAY));
            }
        }
        
        let mut preload:String = "".to_string();
        if let Some(preloadstr) = config_params.config.web_preferences.preload {
             preload=preloadstr;
        }
        
        let window = WindowBuilder::new()
            .build(&event_loop)
            .expect("WindowBuilder::new failed");
   
       
        window.set_title(&config_params.config.title);
        if !&config_params.config.show {
            window.set_visible(false);
        }
        window.set_resizable(config_params.config.resizable);

        if let Some(x) = config_params.config.x {
            if let Some(y) = config_params.config.y {
                let lpos = LogicalPosition::new(x, y);
                let pos:PhysicalPosition<i32> = lpos.to_physical(window.current_monitor().expect("window.current_monitor() failed").scale_factor());
                window.set_outer_position(pos);
            }
        }
        if let Some(width) = config_params.config.width {
            if let Some(height) = config_params.config.height {
                window.set_inner_size(LogicalSize::new(width, height));
            }
        }

        if let Some(icon_path) = config_params.config.icon {
            match fs::read(&icon_path) {
                Ok(ifile) => {
                    match Icon::from_rgba(ifile, 32, 32) {
                        Ok(icon) => {
                            #[cfg(any(
                                target_os = "windows",
                                target_os = "macos",
                                target_os = "ios",
                                target_os = "android"
                            ))]
                            window.set_window_icon(Some(icon));
                        },
                        Err(e) => {
                            warn!("icon could not be built: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("icon file not found: {}, {}", icon_path, e);
                }
            }
        }

        let preload_init = preload.clone();
        let nav_handler_id = id.clone();
        let nav_proxy = proxy.clone();
        let nav_handler = move |page: String| {
            trace!("nav_handler {}", page);
            let _ = nav_proxy.send_event(ElectricoEvents::FrontendNavigate {browser_window_id:nav_handler_id.clone(), page, preload:preload.clone()});
            true
        };
        let preload_file = self.rsrc_dir.clone().join(preload_init.clone());
        let mut preload_script = "".to_string();
        if let Some(add_args) = config_params.config.web_preferences.additional_arguments {
            for arg in add_args {
                preload_script=preload_script+format!("window.__electrico.addArgument('{}');", escape(&arg)).as_str();
            }
        }
        if preload_init.len()>0 {
            match std::fs::read_to_string(preload_file) {
                Ok(preloadfilejs) => {
                    preload_script = preload_script + preloadfilejs.as_str();
                },
                Err(_e) => {
                    error!("cant load preload file {}", preload_init);
                }
            }
        }

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
        let mut webview_builder = WebViewBuilder::new(&window);
    
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        let mut webview_builder = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().unwrap();
            WebViewBuilder::new_gtk(vbox)
        };

        fn module_file_path(path:String) -> String {
            let mut fpath = path;
            if fpath.starts_with("/") {
                fpath = fpath.substring(1, fpath.len()).to_string();
            }
            fpath = fpath.replace("..", "");
            return fpath;
        }

        fn is_electrico_ipc(path:&String) -> bool {
            path.starts_with("/electrico-ipc")
        }
        
        let http_uid = Uuid::new_v4().to_string();
        debug!("frontend create_window - http_uid:{http_uid}");
        let http_port = http_port(&self.http_port);
        let frontendalljs = self.frontendalljs.clone();
        let init_script = format!("window.__http_protocol = {{'http_port':{http_port}, 'http_uid':'{http_uid}'}};
                var __electrico_nonce='{id}'; 
                window.__electrico_preload=function(document, ph){{\nph.before(__electrico_nonce); 
                var window=document.window; var process=window.process; 
                var require=document.window.require;\n{preload_script}\nph.after();\n}};\n
                {frontendalljs}\n
                __electrico_nonce='';\n");
        let webview = webview_builder
            .with_initialization_script(init_script.as_str())
            .with_navigation_handler(nav_handler)
            //.with_proxy_config(wry::ProxyConfig::Http(wry::ProxyEndpoint { host: format!("127.0.0.1"), port: format!("{http_port}") }))
            .with_devtools(true)
            .with_clipboard(true)
            .build().unwrap();
        #[cfg(debug_assertions)]
        webview.open_devtools();
        
        let w_id = window.id().clone();
        self.window_http_ids.insert(http_uid.clone(), config_params.id.clone());
        self.window_ids.insert(window.id().clone(), config_params.id.clone());
        let fwindow: FrontendWindow = FrontendWindow::new(window, webview, w_id, http_uid);
        self.windows.insert(config_params.id.clone(), fwindow);
        self.opened_windows+=1;
    }
    pub fn load_url(&mut self, id:&String, fpath:String) {
        if let Some(window) = self.windows.get_mut(id) {
            if fpath.starts_with("http://") || fpath.starts_with("https://") {
                #[cfg(debug_assertions)]
                window.set_client_path_base(Some(self.rsrc_dir.clone()));
                #[cfg(not(debug_assertions))]
                window.set_client_path_base(None);
                let _ = window.webview.load_url(fpath.as_str());
            } else {
                let mut url=fpath.clone();
                if let None = fpath.find("://") {
                    url = "electrico-file://file/".to_string()+url.as_str();
                }
                let http_port = http_port(&self.http_port);
                if let Some(ix) = url.find("://") {
                    let protocol = url.substring(0, ix);
                    url = format!("http://electrico.localhost:{http_port}/{}@{protocol}/{}", window.http_id, url.substring(ix+3, url.len()));
                }
                if let Some(_) = url.find("?") {
                    url = url+"&electrico_hostpage=true";
                } else {
                    url = url+"?electrico_hostpage=true";
                }
                debug!("load_url url={}", url);
                window.set_client_path_base(Some(self.rsrc_dir.clone()));
                let _ = window.webview.load_url(url.as_str());
            }
        } else {
            error!("load_url - frontend_webview not there - id: {}", id);
        }
    }
    pub fn set_title(&mut self, id:&String, title: String) {
        if let Some(window) = self.windows.get(id) {
            window.window.set_title(title.as_str());
        } else {
            error!("set_title - frontend_webview not there - id: {}", id);
        }
    }
    pub fn get_title(&mut self, id:&String) -> Option<String> {
        if let Some(window) = self.windows.get(id) {
            return Some(window.window.title());
        } else {
            error!("get_title - frontend_webview not there - id: {}", id);
            return None;
        }
    }
    pub fn show(&mut self, id:&String, shown: bool) {
        if let Some(window) = self.windows.get(id) {
            window.window.set_visible(shown);
        } else {
            error!("show - frontend_webview not there - id: {}", id);
        }
    }
    pub fn send_channel_message(&mut self, proxy: EventLoopProxy<ElectricoEvents>, id:String, rid:String, channel:String, args:String, data:Option<Vec<u8>>) {
        if let Some(window) = self.windows.get(&id) {
            if let Some(data) = data {
                self.data_queue.add(&rid, data);
            }
            let _ = window.webview.evaluate_script_with_callback(format!("window.__electrico.sendChannelMessage('{}', '{}', '{}');", &rid, &channel, escape(&args)).as_str(), move |r| {
                if r.len()==0 {
                    trace!("send_channel_message not OK - resending");
                    let _ = proxy.send_event(ElectricoEvents::SendChannelMessageRetry { browser_window_id: id.clone(), rid:rid.clone(), channel:channel.clone(), args:args.clone()});
                }
            });
        } else {
            error!("send_channel_message - frontend_webview not there - id: {}", id);
        }
    }
    pub fn get_data_blob(&mut self, id:String) -> Option<Vec<u8>> {
        let data:Option<Vec<u8>>;
        if let Some(d) = self.data_queue.take(&id) {
            data = Some(d.to_vec());
        } else {
            data = None;
        };
        return data;
    }
    pub fn execute_javascript(&mut self, id:&String, script:&String) {
        if let Some(window) = self.windows.get(id) {
            let _ = window.webview.evaluate_script(script);
        } else {
            error!("execute_javascript - frontend_webview not there - id: {}", id);
        }
    }
    pub fn open_devtools(&mut self, id:&String) {
        if let Some(window) = self.windows.get(id) {
            #[cfg(debug_assertions)]
            window.webview.open_devtools();
        } else {
            error!("open_devtools - frontend_webview not there - id: {}", id);
        }
    }
    pub fn close_devtools(&mut self, id:&String) {
        if let Some(window) = self.windows.get(id) {
            #[cfg(debug_assertions)]
            window.webview.close_devtools();
        } else {
            error!("close_devtools - frontend_webview not there - id: {}", id);
        }
    }
    pub fn get_id(&mut self, id:&WindowId) -> Option<&String> {
        self.window_ids.get(id)
    }
    pub fn get_client_path_base(&mut self, id:&String) -> &Option<PathBuf> {
        if let Some(window) = self.windows.get(id) {
            &window.client_path_base
        } else {
            error!("get_client_path_base - frontend_webview not there - id: {}", id);
            &None
        }
    }
    pub fn content_bounds(&mut self, id:&String) -> Option<Rectangle> {
        if let Some(win) = self.windows.get(id) {
            if let Ok(pos) = win.window.outer_position() {
                let lpos:LogicalPosition<i32> = pos.to_logical(win.window.current_monitor().unwrap().scale_factor());
                let size = win.window.outer_size().to_logical(win.window.current_monitor().unwrap().scale_factor());
                let bounds = Rectangle::new(lpos.x, lpos.y, size.width, size.height);
                return Some(bounds);
            }
        }
        None
    }
    pub fn set_content_bounds(&mut self, id:&String, bounds:Rectangle) {
        if let Some(win) = self.windows.get(id) {
            win.window.set_outer_position(PhysicalPosition::new(bounds.x, bounds.y));
            win.window.set_inner_size(PhysicalSize::new(bounds.width, bounds.height));
        } else {
            error!("set_content_bounds - frontend_webview not there - id: {}", id);
        }
    }
    pub fn is_maximized(&mut self, id:&String) -> bool {
        if let Some(win) = self.windows.get(id) {
            return win.window.is_maximized();
        }
        false
    }
    pub fn set_maximized(&mut self, id:&String, maximized:bool) {
        if let Some(win) = self.windows.get(id) {
            win.window.set_visible(true);
            win.window.set_maximized(maximized);
        }
    }
    pub fn is_minimized(&mut self, id:&String) -> bool {
        if let Some(win) = self.windows.get(id) {
            return win.window.is_minimized();
        }
        false
    }
    pub fn set_minimized(&mut self, id:&String, minimized:bool) {
        if let Some(win) = self.windows.get(id) {
            win.window.set_minimized(minimized);
        }
    }
    pub fn close(&mut self,  event_loop:&EventLoopWindowTarget<ElectricoEvents>, id:&String) {
        if let Some(win) = self.windows.get(id) {
            if self.window_ids.len()>1 {
                self.window_ids.remove(&win.id);
                self.windows.remove(id);
            } else {
                #[cfg(target_os = "macos")] {
                    use tao::platform::macos::EventLoopWindowTargetExtMacOS;
                    event_loop.hide_application();
                }
                #[cfg(not(target_os = "macos"))] {
                    self.window_ids.remove(&win.id);
                    self.windows.remove(id);
                }
            }
            if self.opened_windows>0 {
                self.opened_windows-=1;
            }
        }
    }
    pub fn count(&mut self) -> usize {
        self.opened_windows
    }
    pub fn set_focus(&mut self, id:&String) {
        if let Some(window) = self.windows.get(id) {
            window.window.set_focus();
        }
    }
    pub fn print(&mut self, id:&String) {
        if let Some(window) = self.windows.get(id) {
            let _ = window.webview.print();
        }
    }
    pub fn toggle_dev_tools(&mut self) {
        for (_id, win) in self.windows.iter() {
            if win.window.is_focused() {
                #[cfg(debug_assertions)] {
                    if win.webview.is_devtools_open() {
                        win.webview.close_devtools();
                    } else {
                        win.webview.open_devtools();
                    }
                }
            }
        }
    }
    pub fn get_actual_window(&mut self) -> Option<&Window>  {
        if let Some(win) = self.windows.values().last() {
            return Some(&win.window);
        }
        None
    }
    pub fn get_browser_window_id(&mut self, http_id:&String) -> Option<&String>  {
        return self.window_http_ids.get(http_id);
    }
    pub fn dom_content_loaded(&mut self, id:&String, title:String) {
        if let Some(window) = self.windows.get(id) {
            if title.len()>0 {
                let _ = window.window.set_title(title.as_str());
            }
        } else {
            error!("set_title - frontend_webview not there - id: {}", id);
        }
    }
    pub fn register_file_protocol(&mut self, schema:String) {
        self.file_protocols.push(schema);
    }
    pub fn get_file_protocols(&self) -> &Vec<String> {
        &self.file_protocols
    }
}