use std::{io::{BufRead, BufReader}, process::{Command, Stdio}, time::Duration};
use log::{error, debug, trace};
use reqwest::StatusCode;
use tao::event_loop::EventLoopProxy;
use tokio::{runtime::Runtime, sync::mpsc::{self, Receiver, Sender}, time::timeout};
use std::io::Write;
use wry::RequestAsyncResponder;
use crate::{backend::Backend, common::{respond_client_error, respond_status, CONTENT_TYPE_TEXT}, node::common::send_command, types::{BackendCommand, ChildProcess, ElectricoEvents}};

pub fn child_process_spawn(
    cmd_in:Option<String>, 
    args:Option<Vec<String>>,
    backend:&mut Backend,
    tokio_runtime:&Runtime,
    proxy: EventLoopProxy<ElectricoEvents>, 
    command_sender: std::sync::mpsc::Sender<BackendCommand>,
    responder:RequestAsyncResponder) {
    let mut pargs:Vec<String> = Vec::new();
    if let Some(args) = args {
        pargs = args;
    }
    let cmd:String;
    if let Some(cmd_in) = cmd_in {
        cmd = cmd_in;
    } else {
        #[cfg(unix)] {
            cmd = "sh".to_string();
        }
        #[cfg(not(unix))] {
            cmd = "powershell".to_string();
        }
    }
    match Command::new(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(pargs).spawn() {
        Ok(mut child) => {
            let (sender, mut receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel(100);
            backend.child_process_start(&child.id().to_string(), sender.clone());
            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), child.id().to_string().into_bytes(), responder); 
            tokio_runtime.spawn(
                async move {
                    let stdout;
                    let stderr;
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
                    let blen = 65536;
                    let mut exit_code:Option<i32> = None;
                    let mut disconnected = false;
                    let stderr_proxy = proxy.clone();
                    let stderr_command_sender = command_sender.clone();
                    let stderr_sender = sender.clone();
                    let chid = child.id().to_string();
                    let stderr_chid = chid.clone();
                    let (stderr_end_sender, mut stderr_end_receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel(100);
                    tokio::spawn(async move {
                        trace!("starting stderr {}", stderr_chid);
                        let mut reader = BufReader::with_capacity(blen, stderr);
                        loop {
                            if let Ok(d) = stderr_end_receiver.try_recv() {
                                if let ChildProcess::Disconnect = d {
                                    break;
                                }
                            }
                            if let Ok(b) = reader.fill_buf() {
                                let len = b.len();
                                if len==0 {
                                    break;
                                }
                                send_data(&stderr_chid, "stderr".to_string(), b.to_vec(), &stderr_proxy, &stderr_command_sender);
                                reader.consume(len);
                            } else {
                                error!("stderr read error");
                                break;
                            }
                        }
                        trace!("process stderr end {}", stderr_chid);
                        let _ = stderr_sender.send(ChildProcess::StderrEnd).await;
                    });
                    let stdout_proxy = proxy.clone();
                    let stdout_command_sender = command_sender.clone();
                    let (stdout_end_sender, mut stdout_end_receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel(100);
                    fn send_data(pid:&String, stream:String, buffer:Vec<u8>, proxy: &EventLoopProxy<ElectricoEvents>, sender:&std::sync::mpsc::Sender<BackendCommand>) {
                        let _ = send_command(proxy, sender, BackendCommand::ChildProcessCallback { pid:pid.clone(), stream:stream, data:Some(buffer) });
                    }
                    tokio::spawn(async move {
                        trace!("starting stdout {}", chid);
                        let mut reader = BufReader::with_capacity(blen, stdout);
                        loop {
                            if let Ok(d) = stdout_end_receiver.try_recv() {
                                if let ChildProcess::Disconnect = d {
                                    break;
                                }
                            }
                            if let Ok(b) = reader.fill_buf() {
                                let len = b.len();
                                if len==0 {
                                    break;
                                }
                                send_data(&chid, "stdout".to_string(), b.to_vec(), &stdout_proxy, &stdout_command_sender);
                                reader.consume(len);
                            } else {
                                error!("stdout read error");
                                break;
                            }
                        }
                        trace!("process stdout end {}", chid);
                        let _ = sender.send(ChildProcess::StdoutEnd).await;
                    });
                    tokio::spawn(async move {
                        let mut stdout_end=false;
                        let mut stderr_end=false;
                        loop {
                            if let Ok(cp) = timeout(Duration::from_secs(1), receiver.recv()).await {
                                if let Some(cp) = cp {
                                    match cp {
                                        ChildProcess::StdinWrite { data } => {
                                            trace!("writing stdin {}", data.len());
                                            let _ = stdin.write_all(data.as_slice());
                                        },
                                        ChildProcess::Disconnect => {
                                            let _ = stdout_end_sender.send(ChildProcess::Disconnect).await;
                                            let _ = stderr_end_sender.send(ChildProcess::Disconnect).await;
                                            disconnected = true;
                                            break;
                                        },
                                        ChildProcess::Kill => {
                                            let _ = stdout_end_sender.send(ChildProcess::Disconnect).await;
                                            let _ = stderr_end_sender.send(ChildProcess::Disconnect).await;
                                            disconnected = true;
                                            let _ = child.kill();
                                        },
                                        ChildProcess::StderrEnd => {
                                            stdout_end=true;
                                        },
                                        ChildProcess::StdoutEnd => {
                                            stderr_end=true;
                                        }
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
                                    if stdout_end && stderr_end{
                                        break;
                                    }
                                }
                                if disconnected {
                                    break;
                                }
                            }
                        }
                        trace!("child process exit");
                        if !disconnected {
                            let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:exit_code});
                        }
                    });
                }
            );         
        },
        Err(e) => {
            respond_client_error(format!("Error: {}", e), responder);
        }
    }
}