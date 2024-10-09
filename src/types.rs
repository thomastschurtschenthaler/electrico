use std::sync::mpsc::Sender;
use notify::Event;
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

pub enum ChildProcess {
  StdinWrite {data: Vec<u8>},
  Disconnect,
  StdoutEnd,
  StderrEnd
}

pub enum NETConnection {
  Write {data: Vec<u8>},
  Disconnect,
  EndConnection
}

pub enum NETServer {
  Close
}

pub enum BackendCommand {
  ChildProcessCallback {pid:String, stream:String, data:Option<Vec<u8>>},
  ChildProcessExit {pid:String, exit_code:Option<i32>},
  FSWatchEvent {wid:String, event:Event},
  NETServerStart {id:String, sender:Sender<NETServer>},
  NETServerConnStart {hook:String, id:String, sender:Sender<NETConnection>},
  NETConnectionData {id:String, data:Option<Vec<u8>>},
  NETConnectionEnd {id:String},
  NETClientConnStart {id:String, sender:Sender<NETConnection>},
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
  ExecuteCommand {command: Command, responder: RequestAsyncResponder, data_blob:Option<Vec<u8>>},
  FrontendNavigate {browser_window_id:String, page: String, preload: String},
  IPCCallRetry {browser_window_id:String, request_id:String, params:String, sender:Sender<IPCMsg>},
  SendChannelMessageRetry { browser_window_id:String, channel:String, args:String},
  Exit,
  Noop
}
