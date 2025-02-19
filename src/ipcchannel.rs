use std::{collections::HashMap, sync::mpsc::Sender, time::SystemTime};

use hyper::StatusCode;
use log::{error, debug};

pub struct IPCResponse {
    pub params:Vec<u8>, 
    pub mime_type:String,
    pub status:StatusCode
}

impl IPCResponse {
    pub fn new(params:Vec<u8>, mime_type:String, status:StatusCode) -> IPCResponse {
        IPCResponse {
            params:params,
            mime_type:mime_type,
            status:status,
        }
    }
}

pub struct IPCChannel {
    ipc_channel:HashMap<String, (String, Sender<IPCResponse>)>,
    ipc_channel_timeout:HashMap<String, SystemTime>,
}

impl IPCChannel {
    pub fn new() -> IPCChannel {
        IPCChannel {
            ipc_channel:HashMap::new(),
            ipc_channel_timeout:HashMap::new(),
        }
    }
    pub fn start(&mut self, browserwindow_id: String, k: String, v: Sender<IPCResponse>) {
        self.clean_timeout();   
        self.ipc_channel_timeout.insert(k.clone(), SystemTime::now());
        self.ipc_channel.insert(k, (browserwindow_id, v));
    }
    pub fn get(&mut self, k: &String) ->  Option<&Sender<IPCResponse>> {
        if let Some(c) =  self.ipc_channel.get(k) {
            return Some(&c.1);
        }
        None
    }
    pub fn end(&mut self, k:&String) {
        self.ipc_channel_timeout.remove(k);
        self.ipc_channel.remove(k);
    }
    pub fn clean_timeout(&mut self) {
        for (k, v) in self.ipc_channel_timeout.clone().into_iter() {
            match v.elapsed() {
                Ok(elapsed) => {
                    if elapsed.as_secs()>600 {
                        debug!("ipc_response_clean_timeout request timed out {}", k);
                        self.ipc_channel.remove(&k);
                        self.ipc_channel_timeout.remove(&k); 
                    }
                },
                Err(e) => {
                    error!("ipc_response_clean_timeout SystemTimeError {}", e.to_string());
                }
            }
        }
    }
}