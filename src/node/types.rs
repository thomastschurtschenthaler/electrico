use std::{collections::HashMap, time::SystemTime};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum ConsoleLogLevel {
  Info,
  Debug,
  Warn,
  Error,
  Trace
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConsoleLogParam {
  pub level: ConsoleLogLevel,
  pub logmsg: String,
  pub logdata: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FSOptions {
  pub encoding: Option<String>,
  pub recursive: Option<bool>,
  #[serde(rename = "withFileTypes")]
  pub with_file_types:  Option<bool>
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FSDirent {
  pub path: String,
  pub name: String,
  #[serde(rename = "isDirectory")]
  pub is_directory: bool,
}

impl FSDirent {
  pub fn new(path:String, name: String, is_directory: bool) -> FSDirent {
    FSDirent {path, name, is_directory}
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct HTTPOptions {
  pub hostname: String,
  pub port: i32,
  pub path: String,
  pub method: String
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FSStat {
  #[serde(rename = "isDirectory")]
  pub is_directory: bool,
  pub birthtime: Option<SystemTime>,
  pub mtime: Option<SystemTime>
}
impl FSStat {
  pub fn new(is_directory: bool, birthtime:Option<SystemTime>, mtime:Option<SystemTime>) -> FSStat {
    FSStat { is_directory, birthtime, mtime }
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NETOptions {
  
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "command")]
pub enum NodeCommand {
  ConsoleLog {params: ConsoleLogParam},
  GetProcessInfo,
  GetStartArgs,
  FSAccess {path:String, mode:i32},
  FSLstat {path:String},
  FSMkdir {path:String, options:Option<FSOptions>},
  FSReadFile {path:String, options:Option<FSOptions>},
  FSReadDir {path:String, options:Option<FSOptions>},
  FSWriteFile {path:String, options:Option<FSOptions>},
  FSWatch {path:String, wid:String, options:Option<FSOptions>},
  FSWatchClose {wid:String},
  FSOpen {fd:i64, path:String, flags:String, mode:String},
  FSClose {fd:i64},
  FSRead {fd:i64, offset:i64, length:usize, position:Option<u64>},
  FSWrite {fd:i64, offset:i64, length:usize, position:Option<u64>},
  FSRealPath {path:String},
  FSFdatasync {fd:i64},
  FSUnlink {path:String},
  FSRename {old_path:String, new_path:String},
  NETCreateServer {hook:String, options: Option<NETOptions>},
  NETCloseServer {id:String},
  NETCloseConnection {id:String},
  NETCreateConnection {hook:String, id:String},
  NETWriteConnection {id:String},
  NETSetTimeout {id:String, timeout:u128},
  HTTPRequest {options:HTTPOptions},
  ChildProcessSpawn {cmd: Option<String>, args:Option<Vec<String>>},
  ChildProcessStdinWrite {pid: String},
  ChildProcessDisconnect {pid: String},
  GetDataBlob {id: String},
  Addon {data: String}
}

#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProcessVersions {
  pub node: String,
  pub chrome: String,
  pub electron: String,
}
impl ProcessVersions {
  pub fn new(node: String, chrome: String, electron: String) -> ProcessVersions {
    ProcessVersions {node, chrome, electron}
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Process {
  pub platform: String,
  pub versions: ProcessVersions,
  pub env: HashMap<String, String>,
  #[serde(rename = "resourcesPath")]
  pub resources_path: String,
  #[serde(rename = "execPath")]
  pub exec_path: String,
  pub argv: Vec<String>,
  pub pid: u32
}
impl Process {
  pub fn new(platform:String, versions:ProcessVersions, env: HashMap<String, String>, resources_path: String, exec_path:String, argv: Vec<String>, pid: u32) -> Process {
    Process {platform, versions, env, resources_path, exec_path, argv, pid}
  }
}