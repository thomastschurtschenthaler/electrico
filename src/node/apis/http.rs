use http_body_util::StreamBody;
use hyper::{body::Frame, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use http_body_util::BodyExt;
use reqwest::{header::ACCESS_CONTROL_ALLOW_ORIGIN, Method, Request, StatusCode, Url};
use serde_json::Error;
use tao::event_loop::EventLoopProxy;
use tokio::{io::{AsyncRead, ReadBuf}, net::TcpListener, runtime::Runtime, sync::mpsc::{self, Receiver, Sender}, time::timeout};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use wry::http::Response;
use std::{collections::HashMap, convert::Infallible, net::SocketAddr, pin::Pin, task::{Context, Poll}, time::Duration};
use log::{debug, error};
use futures_util::TryStreamExt;

use crate::{backend::Backend, common::{respond_client_error, respond_ok, respond_status, CONTENT_TYPE_BIN, CONTENT_TYPE_TEXT}, node::{common::send_command, node::AppEnv}, types::{BackendCommand, ElectricoEvents, NETConnection, NETServer, Responder}};

use super::types::HTTPCommand;

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpRequest {
    pub url:String,
    pub path:String,
    pub method:String,
    pub host:String,
    pub protocol:String,
    pub headers:HashMap<String, String>
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpHeader {
    #[serde(rename = "statusCode")]
    pub status_code:String,
    #[serde(rename = "statusMessage")]
    pub status_message:Option<String>,
    pub headers:HashMap<String, String>
}

pub fn process_http_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:HTTPCommand,
    responder:Responder,
    data_blob:Option<Vec<u8>>)  {
    
    match command {
        HTTPCommand::CreateServer { port, options } => {
            let command_sender = backend.command_sender();
            tokio_runtime.spawn(async move {
                let addr = SocketAddr::from(([127, 0, 0, 1], port));
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        let addr: SocketAddr;
                        match listener.local_addr() {
                            Ok(a) => {
                                addr=a;
                            },
                            Err(e) => {
                                respond_client_error(format!("listener.local_addr failed:{e}"), responder);
                                return;
                            }
                        }
                        debug!("listening:{}", addr);
                        let id = addr.port().to_string();
                        let (sender, mut receiver): (Sender<NETServer>, Receiver<NETServer>) = mpsc::channel(100);
                        let _ = send_command(&proxy, &command_sender, BackendCommand::NETServerStart { id: id.clone(), sender:sender });                
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), id.into_bytes(), responder);
                    
                        loop {
                            if let Ok((stream, address)) = listener.accept().await {
                                debug!("http connection from {address} on port {port}");
                                let io = TokioIo::new(stream);
                                let proxy = proxy.clone();
                                let command_sender = command_sender.clone();
                                tokio::task::spawn(async move {
                                    let service = service_fn(move |request:hyper::Request<hyper::body::Incoming>| {
                                        let proxy = proxy.clone();
                                        let command_sender = command_sender.clone();
                                        let id = Uuid::new_v4().to_string();
                                        
                                        return async move {
                                            let (sender, mut receiver): (Sender<NETConnection>, Receiver<NETConnection>) = mpsc::channel(100);
                                            let _ = send_command(&proxy, &command_sender, BackendCommand::NETServerConnStart { hook: format!("{port}"), id:id.clone(), sender:sender});
                                            
                                            let mut headers:HashMap<String, String> = HashMap::new();
                                            for (k,v) in request.headers() {
                                                headers.insert(k.to_string(), v.to_str().unwrap_or_default().to_string());
                                            }
                                            let http_request = HttpRequest {
                                                url:request.uri().to_string(),
                                                path:request.uri().path().to_string(),
                                                method:request.method().to_string(),
                                                host:request.uri().host().unwrap_or_default().to_string(),
                                                protocol:"http".to_string(),
                                                headers:headers
                                            };
                                            match serde_json::to_string(&http_request) {
                                                Ok(json) => {
                                                    let _ = send_command(&proxy, &command_sender, BackendCommand::NETConnectionData {id:id.clone(), data: Some(json.as_bytes().to_vec()) });
                                                },
                                                Err(e) => {
                                                    error!("error json serialize http headers {e}");
                                                }
                                            }
                                            if let Ok(body) = request.collect().await {
                                                let _ = send_command(&proxy, &command_sender, BackendCommand::NETConnectionData {id:id.clone(), data: Some(body.to_bytes().to_vec()) });
                                            }
                                            let mut response_builder = Response::builder();
                                            if let Ok(r) = timeout(Duration::from_secs(30), receiver.recv()).await {
                                                if let Some(c) = r {
                                                    if let NETConnection::Write {data, end} = c {
                                                        let header:Result<HttpHeader, Error> = serde_json::from_str(String::from_utf8(data).unwrap_or_default().as_str());
                                                        match header {
                                                            Ok(h) => {
                                                                debug!("setting headers");
                                                                if let Ok(s) = StatusCode::from_bytes(h.status_code.as_bytes()) {
                                                                    response_builder = response_builder.status(s);
                                                                }
                                                                for (k,v) in h.headers {
                                                                    response_builder = response_builder.header(k, v);
                                                                }
                                                                response_builder = response_builder.header(ACCESS_CONTROL_ALLOW_ORIGIN, "*");
                                                            },
                                                            Err(e) => {
                                                                error!("error json deserialize http headers {e}");
                                                            }
                                                        }
                                                    } else {
                                                        error!("error expected http headers");
                                                    }
                                                } else {
                                                    error!("error no http headers data");
                                                }
                                            } else {
                                                error!("error http response headers timeout");
                                            }
                                           
                                            #[derive(Debug)]
                                            pub struct BodyStream {
                                                receiver:Receiver<NETConnection>,
                                                ended:bool,
                                                buffer:Option<Vec<u8>>
                                            }
                                            fn write_data(data:&Vec<u8>, buf: &mut ReadBuf<'_>) -> Option<Vec<u8>>{
                                                let slice: &[u8];
                                                let buffer: Option<Vec<u8>>;
                                                if data.len()<=buf.remaining() {
                                                    slice = data.as_slice();
                                                    buffer = None;
                                                } else {
                                                    slice = &data.as_slice()[0..buf.remaining()];
                                                    buffer = Some(data.as_slice()[buf.remaining()..data.len()].to_vec());
                                                }
                                                buf.put_slice(slice);
                                                return buffer;
                                            }
                                            impl AsyncRead for BodyStream {
                                                fn poll_read(
                                                    mut self: Pin<&mut Self>,
                                                    cx: &mut Context<'_>,
                                                    buf: &mut ReadBuf<'_>,
                                                ) -> Poll<std::io::Result<()>> {
                                                    if let Some(buffer) = &self.buffer {
                                                        self.buffer = write_data(buffer, buf);
                                                        return Poll::Ready(Ok(()));
                                                    }
                                                    if self.ended {
                                                        return Poll::Ready(Ok(()));
                                                    }
                                                    
                                                    if let Ok(r) = self.receiver.try_recv() {
                                                       if let NETConnection::Write {data, end} = r {
                                                            debug!("NETConnection::Write:{}", end);
                                                            self.buffer = write_data(&data, buf);
                                                            self.ended=end;
                                                            return Poll::Ready(Ok(()));
                                                        }
                                                    }
                                                    cx.waker().wake_by_ref();
                                                    return Poll::Pending;
                                                }
                                            }
                                            let stream = BodyStream {receiver:receiver, ended:false, buffer:None};
                                            let reader_stream = ReaderStream::new(stream);
                                            let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
                                            let boxed_body = stream_body.boxed();
                                            return Ok::<_, Infallible>(response_builder
                                                .body(boxed_body)
                                                .unwrap());
                                            
                                        };
                                    });
                                    if let Err(err) = http1::Builder::new()
                                        .keep_alive(true)
                                        .serve_connection(io, service)
                                        .await
                                    {
                                        error!("HTTPCommand::CreateServer - Error serving connection: {:?}", err);
                                    }
                                });
                            }
                        }
                    },
                    Err(e) => {
                        error!("HTTPCommand::CreateServer bind failed {}", e);
                        respond_client_error(format!("bind failed {}", e), responder);
                    }
                }
            });
        }
        HTTPCommand::Request { options } => {
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
                                        respond_status(StatusCode::OK, CONTENT_TYPE_BIN.to_string(), Vec::from(body), responder);
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
        }
    }
}