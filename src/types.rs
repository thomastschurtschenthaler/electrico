use notify::Event;
use tokio::sync::mpsc::Sender;
use wry::RequestAsyncResponder;
use crate::{electron::types::ElectronCommand, ipcchannel::IPCResponse, node::types::NodeCommand};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Resources {
    pub link: Option<String>
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Package {
    pub main: String,
    pub version: String,
    pub name: String,
    pub fork: Option<(String, String, String)>
}

impl Package {
  pub fn new(main: String, version: String, name: String) -> Package {
    Package {
      main, version, name, fork:None
    }
  }
  pub fn new_fork(main: String, version: String, name: String, hook:String, clientid:String, env:String) -> Package {
    Package {
      main, version, name, fork:Some((hook, clientid, env))
    }
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ForkParams {
  #[serde(rename = "moduleSrc")]
  pub module_src: String,
  #[serde(rename = "moduleMain")]
  pub module_main: String,
  pub hook: String,
  pub clientid: String,
  pub args: Vec<String>,
  pub env: String
}

pub enum ChildProcess {
  StdinWrite {data: Option<Vec<u8>>, end:bool},
  Disconnect,
  Kill,
  StdoutEnd,
  StderrEnd
}

pub enum NETConnection {
  Write {data: Vec<u8>, end:bool},
  SetTimeout {timeout: Option<u128>},
  Disconnect,
  EndConnection
}

pub enum NETServer {
  Close
}

pub struct ChannelMsg {
  pub channel: String,
  pub params: String,
  pub data_blob: Option<Vec<u8>>
}
pub enum WebSocketCmd {
  
}

pub enum BackendCommand {
  ChildProcessCallback {pid:String, stream:String, end:bool, data:Option<Vec<u8>>},
  ChildProcessExit {pid:String, exit_code:Option<i32>},
  FSWatchEvent {wid:String, event:Event},
  NETServerStart {id:String, sender:Sender<NETServer>},
  NETServerConnStart {hook:String, id:String, sender:Sender<NETConnection>},
  NETConnectionData {id:String, data:Option<Vec<u8>>},
  NETConnectionEnd {id:String},
  NETClientConnStart {id:String, sender:Sender<NETConnection>},
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Command {
  PostIPC {http_id: String, from_backend:bool, nonce:Option<String>, request_id:String, channel:String, params: String},
  SetIPCResponse {request_id:String, file_path:Option<String>},
  DOMContentLoaded {http_id: String, title:String},
  BrowserWindowReadFile {http_id: String, file_path: String, module:bool},
  Node {invoke:NodeCommand},
  Electron {invoke:ElectronCommand},
  ShellCallback {stdout:String},
  FrontendGetProcessInfo {http_id:String, nonce: String},
  FrontendGetProtocols
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CommandMessage {
  pub command: Command,
  pub data_blob:bool
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum FrontendCommand {
  PostIPC {request_id:String, nonce:String, data_blob:bool, channel:String, params: String},
  GetProcessInfo {nonce:String},
  DOMContentLoaded {title: String},
  Alert {message: String},
  GetProtocols
}

pub enum Responders {
  HTTP {sender:Sender<IPCResponse>},
  CP {responder:RequestAsyncResponder}
}

pub enum Responder {
  CustomProtocol {responder:RequestAsyncResponder},
  HttpProtocol {sender:Sender<IPCResponse>},
  None
}

pub enum ElectricoEvents {
  ExecuteCommand {command: Command, responder: Responder, data_blob:Option<Vec<u8>>},
  FrontendNavigate {browser_window_id:String, page: String, preload: String},
  FrontendConnectWS {http_id:String, window_id:String, channel:String, ws_sender:Sender<WebSocketCmd>, msg_sender:Sender<ChannelMsg>},
  BackendConnectWS {channel:String, msg_sender:Sender<ChannelMsg>},
  Exit,
  Noop
}
