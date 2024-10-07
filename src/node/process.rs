use std::{process::{Command, Stdio}, sync::mpsc::{self, Sender, Receiver}, thread};

use log::{error, trace};
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
            backend.child_process_start(child.id().to_string(), sender);
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
                    loop {
                        if let Ok(cp) = receiver.try_recv() {
                            match cp {
                                ChildProcess::StdinWrite { data } => {
                                    trace!("writing stdin {}", data.len());
                                    let _ = stdin.write(data.as_slice());
                                },
                                ChildProcess::Disconnect => {
                                    trace!("disconnect");
                                    break;
                                }
                            }
                        }
                        let mut stdinread:usize=0;
                        let mut stderrread:usize=0;
                        let stdout_buf:&mut [u8] = &mut [0; 1024];
                        if let Ok(read) = stdout.read(stdout_buf) {
                            trace!("stdout read {}", read);
                            stdinread = read;
                            if read>0 {
                                let data:Vec<u8> = stdout_buf[0..read].to_vec();
                                let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessCallback { pid:child.id().to_string(), stream:"stdout".to_string(), data:Some(data) });
                            }
                        }
                        let stderr_buf:&mut [u8] = &mut [0; 1024];
                        if let Ok(read) = stderr.read(stderr_buf) {
                            trace!("stderr read {}", read);
                            stderrread = read;
                            if read>0 {
                                let data:Vec<u8> = stderr_buf[0..read].to_vec();
                                let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessCallback { pid:child.id().to_string(), stream:"stderr".to_string(), data:Some(data) });
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
                            if stdinread==0 && stderrread==0 {
                                break;
                            }
                        }
                        thread::yield_now();
                    }
                    let _ = send_command(&proxy, &command_sender, BackendCommand::ChildProcessExit {pid:child.id().to_string(), exit_code:exit_code});
                }
            );         
        },
        Err(e) => {
            respond_client_error(format!("Error: {}", e), responder);
        }
    }
}