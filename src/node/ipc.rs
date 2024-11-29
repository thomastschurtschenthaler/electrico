use std::{fs, path::Path, time::{Duration, SystemTime}};

use interprocess::local_socket::{ToFsName, traits::tokio::{Listener, Stream}, GenericFilePath, ListenerOptions};
use reqwest::StatusCode;
use tao::event_loop::EventLoopProxy;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, runtime::Runtime, sync::mpsc::{self, Receiver, Sender}, time::{sleep, timeout}};
use log::{debug, error, trace};
use uuid::Uuid;
use wry::RequestAsyncResponder;

use crate::{common::{respond_404, respond_ok, respond_status, CONTENT_TYPE_TEXT}, node::common::send_command, types::{BackendCommand, ElectricoEvents, NETConnection, NETServer}};

pub fn ipc_server(
        hook:String, 
        tokio_runtime:&Runtime, 
        proxy: EventLoopProxy<ElectricoEvents>, 
        command_sender: std::sync::mpsc::Sender<BackendCommand>,
        responder:RequestAsyncResponder) {
    if let Ok(name) = hook.clone().to_fs_name::<GenericFilePath>() {
        let lo = ListenerOptions::new().name(name);
        let s_hook = hook.clone();
        let s_proxy = proxy.clone();
        let s_command_sender = command_sender.clone();

        #[cfg(unix)] {
            if Path::new(hook.as_str()).exists() {
                trace!("removing socket file {}", hook);
                let _ = fs::remove_file(hook);
            }
        }
        tokio_runtime.spawn(async move {
           match lo.create_tokio() {
                Ok(l) => {
                    let id = Uuid::new_v4().to_string();
                    let (sender, mut receiver): (Sender<NETServer>, Receiver<NETServer>) = mpsc::channel(100);
                    let _ = send_command(&s_proxy, &s_command_sender, BackendCommand::NETServerStart { id: id.clone(), sender:sender });
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), id.into_bytes(), responder);
                    loop {
                        match timeout(Duration::from_secs(5), l.accept()).await {
                            Ok(rc) => {
                                match rc {
                                    Ok(c) => {
                                        let id = Uuid::new_v4().to_string();
                                        trace!("ipc listener connection start id {}", id);
                                        let (sender, receiver): (Sender<NETConnection>, Receiver<NETConnection>) = mpsc::channel(100);
                                        let _ = send_command(&s_proxy, &s_command_sender, BackendCommand::NETServerConnStart { hook: s_hook.clone(), id:id.clone(), sender:sender});
                                        ipc_connect(&id, c, receiver, proxy.clone(), command_sender.clone());
                                    },
                                    Err(e) => {
                                        error!("ipc listener error {}", e);
                                    }
                                }
                            },
                            Err(_t) => {
                                if let Ok(c) = receiver.try_recv() {
                                    match c {
                                        NETServer::Close => {
                                            trace!("NETServer close");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    trace!("NETServer closed");
                },
                Err(e) => {
                    error!("NETCreateServer Error {}", e);
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).to_string().into_bytes(), responder); 
                }
            }
        });
    } else {
        respond_404(responder);
    }
}

pub fn ipc_connection(
        hook:String,
        id:String,
        tokio_runtime:&Runtime, 
        proxy: EventLoopProxy<ElectricoEvents>, 
        command_sender: std::sync::mpsc::Sender<BackendCommand>,
        responder:RequestAsyncResponder) {
    if let Ok(name) = hook.clone().to_fs_name::<GenericFilePath>() {
        let c_proxy = proxy.clone();
        let c_command_sender = command_sender.clone();
        let (sender, receiver): (Sender<NETConnection>, Receiver<NETConnection>) = mpsc::channel(100);
        tokio_runtime.spawn(async move {
            let _ = send_command(&c_proxy, &c_command_sender, BackendCommand::NETClientConnStart { id:id.clone(), sender:sender});
            match interprocess::local_socket::tokio::Stream::connect(name).await {
                Ok(c) => {
                    respond_ok(responder);
                    ipc_connect(&id, c, receiver, proxy, command_sender);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).to_string().into_bytes(), responder);
                }
            }
        });
    } else {
        respond_404(responder);
    }
}

enum SendTimeout {
    Timedout,
    SetTimout {timeout: Option<u128>}
}

fn ipc_connect(id:&String, c:interprocess::local_socket::tokio::Stream, 
            mut receiver:Receiver<NETConnection>,
            proxy: EventLoopProxy<ElectricoEvents>,
            command_sender: std::sync::mpsc::Sender<BackendCommand>) {

    let r_proxy = proxy.clone();
    let r_command_sender = command_sender.clone();
    let r_id=id.clone();

    let (mut reader, mut writer) = c.split();
    //let mut bw = BufWriter::new(writer);
    let mut br = BufReader::new(reader);
    let w_proxy = proxy.clone();
    let w_command_sender = command_sender.clone();
    let w_id=id.clone();
    let (timeout_sender, mut timeout_receiver): (Sender<SendTimeout>, Receiver<SendTimeout>) = mpsc::channel(100);
    
    tokio::spawn(async move {
        let mut time_out:Option<u128> = None;
        let mut time_idle = SystemTime::now();
        loop {
            trace!("NETConnection::write loop {}", w_id);
            match timeout(Duration::from_secs(1), receiver.recv()).await {
                Ok(r) => {
                    if let Some(r) =r {
                        match r {
                            NETConnection::Write { data } => {
                                trace!("NETConnection::Write {}: {}", w_id, data.len());
                                time_idle = SystemTime::now();
                                let _ = writer.write_all(&data.to_vec()).await;
                            },
                            NETConnection::SetTimeout { timeout } => {
                                time_out = timeout;
                                let _ = timeout_sender.send(SendTimeout::SetTimout { timeout });
                            },
                            NETConnection::Disconnect => {
                                trace!("NETConnection::Disconnect {}", w_id);
                                break;
                            },
                            NETConnection::EndConnection => {
                                trace!("NETConnection::EndConnection {}", w_id);
                                let _ = send_command(&w_proxy, &w_command_sender, BackendCommand::NETConnectionEnd { id:w_id.clone() });
                            }
                        }
                    }
                },
                Err(_e) => {
                    if let Some(timeout) = time_out {
                        if let Ok(elapsed) = time_idle.elapsed() {
                            if elapsed.as_millis()>timeout {
                                trace!("NETConnection::receive_timeout {}", w_id);
                                let _ = timeout_sender.send(SendTimeout::Timedout);
                            }
                        }
                    }
                }
            }
        }
        trace!("NETConnection write end {}", w_id);
    });
    //let mut buffer:Vec<u8> = vec![0; 65536];
    tokio::spawn(async move {
        let mut time_out:Option<u128> = None;
        let mut time_idle = SystemTime::now();
        loop {
            trace!("NETConnection::read loop {}", r_id);
            match timeout(Duration::from_secs(1),  br.fill_buf()).await {
                Ok(r) => {
                    match r {
                        Ok(b) => {
                            let read = b.len();
                            if read>0 {
                                trace!("NETConnection bytes read {}: {}", r_id, read);
                                time_idle = SystemTime::now();
                                let _ = send_command(&r_proxy, &r_command_sender, BackendCommand::NETConnectionData {id:r_id.clone(), data: Some(b.to_vec()) });
                                br.consume(read);
                            } else {
                                trace!("NETConnection stream end {}", r_id);
                                sleep(Duration::from_millis(100)).await;
                                let _ = send_command(&r_proxy, &r_command_sender, BackendCommand::NETConnectionEnd { id:r_id.clone() });
                                break;
                            }
                        },
                        Err(e) => {
                            error!("ipc NETConnection stream read error {}", e);
                        }
                    }
                },
                Err(_e) => {
                    trace!("NETConnection::read_timeout {}", r_id);
                    if let Ok(_t) = timeout_receiver.try_recv() {
                        trace!("read and write timeout");
                        let _ = send_command(&r_proxy, &r_command_sender, BackendCommand::NETConnectionEnd { id:r_id.clone() });
                        break;
                    }
                }
            }
            if let Ok(st) = timeout_receiver.try_recv() {
                match st {
                    SendTimeout::SetTimout { timeout } => {
                        time_out = timeout;
                    },
                    SendTimeout::Timedout => {
                        if let Some(timeout) = time_out {
                            if let Ok(elapsed) = time_idle.elapsed() {
                                if elapsed.as_millis()>timeout {
                                    trace!("write and read timeout");
                                    let _ = send_command(&r_proxy, &r_command_sender, BackendCommand::NETConnectionEnd { id:r_id.clone() });
                                    break;
                                }
                            }
                        }
                        trace!("write timeout, but no read timeout")
                    }
                }
            }
        }
        trace!("NETConnection read end {}", r_id);
    });
}