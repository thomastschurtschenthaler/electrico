use std::sync::mpsc::Sender;
use wry::RequestAsyncResponder;
use crate::{electron::types::ElectronCommand, ipcchannel::IPCMsg, node::types::NodeCommand};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Resources {
    pub link: Option<String>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Package {
    pub main: String,
    pub version: String,
    pub name: String
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum Command {
  PostIPC {browser_window_id: String, request_id:String, params: String},
  SetIPCResponse {request_id:String, params: String},
  DOMContentLoaded {browser_window_id: String, title:String},
  BrowserWindowReadFile {browser_window_id: String, file_path: String, module:bool},
  Node {invoke:NodeCommand},
  Electron {invoke:ElectronCommand},
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum FrontendCommand {
  PostIPC {request_id:String, nonce:String, params: String},
  GetProcessInfo {nonce:String},
  DOMContentLoaded {title: String},
  Alert {message: String}
}

pub enum ElectricoEvents {
  ExecuteCommand {command: Command, responder: RequestAsyncResponder},
  FrontendNavigate {browser_window_id:String, page: String, preload: String},
  IPCCallRetry {browser_window_id:String, request_id:String, params:String, sender:Sender<IPCMsg>},
  SendChannelMessageRetry { browser_window_id:String, channel:String, args:String},
  Exit,
  ChildProcessData {pid:String, stream:String, data:Vec<u8>},
  ChildProcessStart {pid:String, sender:Sender<Vec<u8>>},
  ChildProcessExit {pid:String, exit_code:Option<i32>}
}
