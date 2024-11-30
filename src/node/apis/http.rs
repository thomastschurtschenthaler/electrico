use reqwest::{header::ACCESS_CONTROL_ALLOW_ORIGIN, Method, Request, StatusCode, Url};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::{http::Response, RequestAsyncResponder};

use crate::{backend::Backend, common::{respond_client_error, respond_ok}, node::node::AppEnv, types::ElectricoEvents};

use super::types::HTTPCommand;

pub fn process_http_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:HTTPCommand,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    
    match command {
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
        }
    }
}