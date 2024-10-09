use std::{process::{Command, Stdio}, sync::mpsc::{self, Receiver, Sender}, thread, time::Duration};

use log::{debug, error, trace};
use reqwest::StatusCode;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;
use std::io::{Read, Write};

use crate::{backend::Backend, common::{respond_client_error, respond_status, CONTENT_TYPE_TEXT}, node::common::send_command, types::{BackendCommand, ChildProcess, ElectricoEvents}};

pub fn child_process_spawn(
        cmd:String, 
        args:Option<Vec<String>>,
        backend:&mut Backend,
        tokio_runtime:&Runtime,
        proxy: EventLoopProxy<ElectricoEvents>, 
        command_sender: Sender<BackendCommand>,
        responder:RequestAsyncResponder) {
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
            let (sender, receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel();
            backend.child_process_start(child.id().to_string(), sender.clone());
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
                    let stderr_proxy = proxy.clone();
                    let stderr_command_sender = command_sender.clone();
                    let stderr_sender = sender.clone();
                    let chid = child.id().to_string();
                    let stderr_chid = chid.clone();
                    let (stderr_end_sender, stderr_end_receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel();
                    tokio::spawn(async move {
                        trace!("starting stderr");
                        loop {
                            let stderr_buf:&mut [u8] = &mut [0; 65536];
                            if let Ok(read) = stderr.read(stderr_buf) {
                                trace!("stderr read {}", read);
                                if let Ok(d) = stderr_end_receiver.try_recv() {
                                    match d {
                                        ChildProcess::Disconnect => {
                                            break;
                                        },
                                        _ => ()
                                    }
                                }
                                if read>0 {
                                    let data:Vec<u8> = stderr_buf[0..read].to_vec();
                                    let _ = send_command(&stderr_proxy, &stderr_command_sender, BackendCommand::ChildProcessCallback { pid:stderr_chid.clone(), stream:"stderr".to_string(), data:Some(data) });
                                } else {
                                    let _ = stderr_sender.send(ChildProcess::StderrEnd);
                                    break;
                                }
                            }
                        }
                    });
                    let stdout_proxy = proxy.clone();
                    let stdout_command_sender = command_sender.clone();
                    let (stdout_end_sender, stdout_end_receiver): (Sender<ChildProcess>, Receiver<ChildProcess>) = mpsc::channel();
                    tokio::spawn(async move {
                        trace!("starting stdoud");
                        loop {
                            let stdout_buf:&mut [u8] = &mut [0; 65536];
                            if let Ok(read) = stdout.read(stdout_buf) {
                                trace!("stdout read {}", read);
                                if let Ok(d) = stdout_end_receiver.try_recv() {
                                    match d {
                                        ChildProcess::Disconnect => {
                                            break;
                                        },
                                        _ => ()
                                    }
                                }
                                if read>0 {
                                    let data:Vec<u8> = stdout_buf[0..read].to_vec();
                                    let _ = send_command(&stdout_proxy, &stdout_command_sender, BackendCommand::ChildProcessCallback { pid:chid.clone(), stream:"stdout".to_string(), data:Some(data) });
                                } else {
                                    let _ = sender.send(ChildProcess::StdoutEnd);
                                    break;
                                }
                            }
                        }
                    });
                    tokio::spawn(async move {
                        let mut stdout_end=false;
                        let mut stderr_end=false;
                        loop {
                            if let Ok(cp) = receiver.recv_timeout(Duration::from_millis(100)) {
                                match cp {
                                    ChildProcess::StdinWrite { data } => {
                                        trace!("writing stdin {}", data.len());
                                        let _ = stdin.write(data.as_slice());
                                    },
                                    ChildProcess::Disconnect => {
                                        let _ = stdout_end_sender.send(ChildProcess::Disconnect);
                                        let _ = stderr_end_sender.send(ChildProcess::Disconnect);
                                        break;
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
                        }
                        trace!("child process exit");
                        let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:exit_code});
                    });
                }
            );         
        },
        Err(e) => {
            respond_client_error(format!("Error: {}", e), responder);
        }
    }
}