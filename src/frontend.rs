use std::convert::Infallible;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use bytes::{BufMut, BytesMut};
use fastwebsockets::{upgrade, Frame, OpCode, Payload};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use log::{debug, error, info, trace, warn};
use include_dir::{include_dir, Dir};
use reqwest::StatusCode;
use substring::Substring;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::timeout;
use uuid::Uuid;
use std::{collections::HashMap, fs, path::PathBuf};
use tao::{dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize}, event_loop::{EventLoopProxy, EventLoopWindowTarget}, window::{Icon, Window, WindowBuilder, WindowId}};
use serde_json::Error;
use wry::{WebView, WebViewBuilder};
use crate::common::{get_message_data_http, parse_http_url_path, CONTENT_TYPE_HTML};
use crate::ipcchannel::IPCResponse;
use crate::types::{ChannelMsg, WebSocketCmd};
use crate::{common::{append_js_scripts, escape, CONTENT_TYPE_TEXT, JS_DIR_FRONTEND}, electron::types::{BrowserWindowCreateParam, Rectangle}, types::{Command, ElectricoEvents, FrontendCommand}};

pub struct FrontendWindow {
    window:Window,
    webview:WebView,
    id:WindowId,
    http_id: String,
    client_path_base:Option<PathBuf>,
    ws_channels:HashMap<String, Sender<WebSocketCmd>>,
    msg_channels:HashMap<String, Sender<ChannelMsg>>
}
impl FrontendWindow {
    pub fn new(window:Window, webview:WebView, id:WindowId, http_id:String) -> FrontendWindow {
        FrontendWindow {
            window:window,
            webview:webview,
            id:id,
            http_id,
            client_path_base:None,
            ws_channels:HashMap::new(),
            msg_channels:HashMap::new()
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
            http_port:None,
            file_protocols:Vec::new()
        }
    }


    #[tokio::main]
    pub async fn start_http_server(proxy:EventLoopProxy<ElectricoEvents>, tcp_listener:TcpListener, port:u16, protocols:Vec<String>) {
        fn is_electrico_ipc(path:&String) -> bool {
            path.starts_with("/electrico-ipc")
        }
        fn module_file_path(path:String) -> String {
            let mut fpath = path;
            if fpath.starts_with("/") {
                fpath = fpath.substring(1, fpath.len()).to_string();
            }
            fpath = fpath.replace("..", "");
            return fpath;
        }
        fn handle_async_ws_ipc(http_id:String, proxy:EventLoopProxy<ElectricoEvents>, request_id:String, nonce:String, channel:String, params:String, data_blob:Option<Vec<u8>>) {
            trace!("frontend handle_async_ws_ipc: {} {} {}", request_id, nonce, params);
            let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::PostIPC {http_id:http_id.clone(), from_backend:false, nonce:Some(nonce), request_id, channel, params}, responder:crate::types::Responder::None, data_blob});
        }
        async fn handle_electrico_ipc(http_id: &String, proxy: &EventLoopProxy<ElectricoEvents>, url:String, query:Option<&str>, request: Vec<u8>, sender:Sender<IPCResponse>) {
            trace!("frontend ipc request {}", url);
            let message_data:Option<(String, Option<Vec<u8>>)> = get_message_data_http(query, request);
            if let Some(message_data) = message_data {
                trace!("frontend ipc request action {}", message_data.0.as_str());
                let commandr:Result<FrontendCommand, Error> = serde_json::from_str(message_data.0.as_str());
                match commandr {
                    Ok (command) => {
                        match command {
                            FrontendCommand::PostIPC {request_id, nonce, data_blob:_, channel, params } => {
                                trace!("frontend ipc call {} {} {}", request_id, nonce, params);
                                let _ = proxy.send_event(ElectricoEvents::ExecuteCommand{command:Command::PostIPC {http_id:http_id.clone(), from_backend:false, nonce:Some(nonce), request_id, channel, params}, responder:crate::types::Responder::HttpProtocol { sender }, data_blob:message_data.1});
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
                thread::spawn(move || {
                    let _ = sender.blocking_send(IPCResponse::new("data error".to_string().as_bytes().to_vec(), CONTENT_TYPE_TEXT.to_string(), StatusCode::BAD_REQUEST));
                });
            }
        }
        trace!("start_http_server");
        let listener = tokio::net::TcpListener::from_std(tcp_listener).expect("TcpListener::from_std failed");
        loop {
            let (stream, _) = listener.accept().await.expect("start_http_server listener.accept() failed");
            let io = TokioIo::new(stream);
            let s_proxy = proxy.clone();
            let protocols = protocols.clone();
            tokio::task::spawn(async move {
                let protocols = protocols.clone();
                let service = service_fn(|mut request:Request<hyper::body::Incoming>| {
                    trace!("http server - got request: {}, {:?}", request.uri().to_string(), request.uri().authority());
                    let urlparts = parse_http_url_path(request.uri().path());
                    let s2_proxy = s_proxy.clone();
                    let protocols = protocols.clone();
                    return async move {
                        if let Some((http_id, protocol, url_root, url)) = urlparts {
                            trace!("http_server processing request:{http_id}, {protocol}, {url_root}, {url}");
                            if protocol == "asyncin" || protocol == "asyncout" {
                                let (ws_sender, _): (Sender<WebSocketCmd>, Receiver<WebSocketCmd>) = mpsc::channel(1000);
                                let (msg_sender, mut msg_receiver): (Sender<ChannelMsg>, Receiver<ChannelMsg>) = mpsc::channel(10000);
                    
                                trace!("async websocket:{},{}", protocol, url_root);
                                let (resp, fut) = upgrade::upgrade(&mut request).expect("upgrade::upgrade failed");
                                tokio::task::spawn(async move {
                                    let socket = fut.await.expect("FragmentCollector failed");
                                    let mut ws = fastwebsockets::FragmentCollector::new(socket);
                                    let ws_channel = url_root;
                                    if let Some(ix) = url.rfind("/") {
                                        let _ = s2_proxy.send_event(ElectricoEvents::FrontendConnectWS {http_id:http_id.clone(), window_id:url.substring(1, ix).to_string(), channel:ws_channel, ws_sender:ws_sender.clone(), msg_sender:msg_sender});
                                    }
                                    if protocol == "asyncin" {
                                        tokio::task::unconstrained(async move {
                                            let mut request_message_data:Option<(String, String, String, String)> = None;
                                            loop {
                                                let http_id = http_id.clone();
                                                let s2_proxy = s2_proxy.clone();
                                                if let Ok(frame) =  ws.read_frame().await {
                                                    match frame.opcode {
                                                        OpCode::Close => {
                                                            debug!("websocket in closed");
                                                            break;
                                                        },
                                                        OpCode::Binary => {
                                                            trace!("websocket message");
                                                            let req_msg:Vec<u8> = frame.payload.into();
                                                            if let Some(msg) = request_message_data {
                                                                request_message_data = None;
                                                                handle_async_ws_ipc(http_id, s2_proxy,  msg.0, msg.1, msg.2,msg.3, Some(req_msg));
                                                            } else {
                                                                let commandr:Result<FrontendCommand, Error> = serde_json::from_slice(req_msg.as_slice());
                                                                match commandr {
                                                                    Ok (command) => {
                                                                        match command {
                                                                            FrontendCommand::PostIPC {request_id, nonce, data_blob, channel, params } => {
                                                                                trace!("frontend ipc call {} {} {}", request_id, nonce, params);
                                                                                if data_blob {
                                                                                    request_message_data = Some((request_id, nonce, channel, params));
                                                                                } else {
                                                                                    handle_async_ws_ipc(http_id, s2_proxy, request_id, nonce, channel, params, None);
                                                                                }
                                                                            },
                                                                            _ => {}
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("handle_async_ws_ipc deserialize:{e}");
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }).await;
                                    } else {
                                        loop {
                                            if let Ok(resp_msg) = timeout(Duration::from_millis(1000), msg_receiver.recv()).await {
                                                if let Some(resp_msg) = resp_msg {
                                                    trace!("websocket write response message");
                                                    let has_data = resp_msg.data_blob.is_some();
                                                    let channel = resp_msg.channel;
                                                    let params = resp_msg.params;
                                                    let message = format!("{has_data}|{channel}|{params}");
                                                    let mut buf = BytesMut::with_capacity(message.len());
                                                    buf.put(message.as_bytes());
                                                    ws.write_frame(Frame::binary(Payload::Bytes(buf))).await.expect("ws.write_frame failed");
                                                    if let Some(data) = resp_msg.data_blob {
                                                        let mut buf = BytesMut::with_capacity(data.len());
                                                        buf.put(data.as_slice());
                                                        ws.write_frame(Frame::binary(Payload::Bytes(buf))).await.expect("ws.write_frame data failed");
                                                    }
                                                }
                                            }
                                            if let Ok(frame) = timeout(Duration::from_millis(1), ws.read_frame()).await {
                                                if let Ok(frame) = frame {
                                                    if frame.opcode == OpCode::Close {
                                                        debug!("websocket out closed");
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                                return Ok(Response::builder()
                                    .status(hyper::StatusCode::SWITCHING_PROTOCOLS)
                                    .header(hyper::header::CONNECTION, "upgrade")
                                    .header(hyper::header::UPGRADE, "websocket")
                                    .header("Sec-WebSocket-Accept", resp.headers().get("Sec-WebSocket-Accept").expect("Sec-WebSocket-Accept header failed"))
                                    .body(Full::new(Bytes::from(""))).expect("http body full failed")); 
                                
                            }
                            let (req_sender, mut req_receiver): (Sender<IPCResponse>, Receiver<IPCResponse>) = mpsc::channel(1000);
                            let url2 = url.clone();
                            if let Ok(body) = request.body_mut().collect().await {
                                let query = request.uri().query().clone();
                                let data = body.to_bytes().to_vec();
                                if is_electrico_ipc(&url) {
                                    trace!("http_server - electrico-ipc request:{url}");
                                    handle_electrico_ipc(&http_id, &s2_proxy, url, query.clone(), data, req_sender).await;
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
                                            from_backend: false,
                                            request_id:Uuid::new_v4().to_string(),
                                            channel: format!("__electrico_protocol"),
                                            params: format!("[\"{}\", \"{}\"]", escape(&protocol), path)
                                        }, responder:crate::types::Responder::HttpProtocol { sender:req_sender}, data_blob:None});
                                    }
                                }
                                if let Ok(r) = timeout(Duration::from_secs(300), req_receiver.recv()).await {
                                    if let Some(r) = r {
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
                                        return Ok::<_, Infallible>(Response::builder()
                                            .header("Content-Type", r.mime_type)
                                            .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                            .status(r.status)
                                            .body(Full::new(Bytes::from(r_body))).expect("http body full failed"));
                                    } else {
                                        error!("http - request no response:{url2}");
                                    }
                                } else {
                                    error!("http - request timed out:{url2}");
                                }
                            }
                        }
                        error!("http - invalid http request");
                        return Ok::<_, Infallible>(Response::builder().status(StatusCode::BAD_REQUEST).body(Full::new(Bytes::from(StatusCode::BAD_REQUEST.to_string()))).expect("http body full failed"));
                    };
                });
                if let Err(err) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }   
    }
    
    pub fn create_window(&mut self, id:String, event_loop:&EventLoopWindowTarget<ElectricoEvents>, proxy:EventLoopProxy<ElectricoEvents>, config_params:BrowserWindowCreateParam) {    
        if self.http_port==None {
            let s_proxy = proxy.clone();
            let protocols = self.file_protocols.clone();
            let listener= TcpListener::bind("127.0.0.1:0").expect("start_http_server - TcpListener failed");
            let addr = listener.local_addr().expect("listener.local_addr failed");
            let _ = listener.set_nonblocking(true);
            thread::spawn(move || {
                Frontend::start_http_server(s_proxy, listener, addr.port(), protocols);
            });
            self.http_port=Some(addr.port());
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

        let mut webview_builder = WebViewBuilder::new();
        
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
        let builder = webview_builder
            .with_initialization_script(init_script.as_str())
            .with_navigation_handler(nav_handler)
            .with_devtools(true)
            .with_clipboard(true);
        
        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        let webview = builder.build(&window).unwrap();
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        let webview = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().unwrap();
            builder.build_gtk(vbox).unwrap()
        };

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
    pub fn send_channel_message(&mut self, id:String, channel:String, args:String, data:Option<Vec<u8>>) {
        if let Some(window) = self.windows.get(&id) {
            if let Some(sender) = window.msg_channels.get("ipcout") {
                let _ = sender.blocking_send(ChannelMsg {channel, params:args, data_blob:data});
            } else {
                error!("send_channel_message - ipcout websocket not there");
            }
        } else {
            error!("send_channel_message - frontend_webview not there - id: {}", id);
        }
    }
    pub fn connect_ws(&mut self, id:&String, channel:String, ws_sender:Sender<WebSocketCmd>, msg_sender:Sender<ChannelMsg>) {
        if let Some(window) = self.windows.get_mut(id) {
            window.ws_channels.insert(channel.clone(), ws_sender);
            window.msg_channels.insert(channel, msg_sender);
        } else {
            error!("connect_ws - frontend_webview not there - id: {}", id);
        }
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