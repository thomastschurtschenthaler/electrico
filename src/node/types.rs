use std::time::SystemTime;

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
  pub name: String,
  #[serde(rename = "isDirectory")]
  pub is_directory: bool,
  #[serde(rename = "isFile")]
  pub is_file: bool,
}

impl FSDirent {
  pub fn new(name: String, is_directory: bool) -> FSDirent {
    FSDirent {name, is_directory, is_file:!is_directory}
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
  NETCreateServer {hook:String, options: Option<NETOptions>},
  NETCloseServer {id:String},
  NETCloseConnection {id:String},
  NETCreateConnection {hook:String, id:String},
  NETWriteConnection {id:String},
  HTTPRequest {options:HTTPOptions},
  ChildProcessSpawn {cmd: String, args:Option<Vec<String>>},
  ChildProcessStdinWrite {pid: String},
  ChildProcessDisconnect {pid: String},
  GetDataBlob {id: String}
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
pub struct ProcessEnv {
  #[serde(rename = "NODE_ENV")]
  pub node_env: String,
  #[serde(rename = "ELECTRON_IS_DEV")]
  pub electron_is_dev: String,
  #[serde(rename = "HOME")]
  pub home: String,
}
impl ProcessEnv {
  pub fn new(node_env: String, electron_is_dev: String, home: String) -> ProcessEnv {
    ProcessEnv {node_env, electron_is_dev, home}
  }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Process {
  pub platform: String,
  pub versions: ProcessVersions,
  pub env: ProcessEnv,
  #[serde(rename = "resourcesPath")]
  pub resources_path: String
}
impl Process {
  pub fn new(platform:String, versions:ProcessVersions, env: ProcessEnv, resources_path: String) -> Process {
    Process {platform, versions, env, resources_path}
  }
}