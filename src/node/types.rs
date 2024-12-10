use std::{collections::HashMap, usize};

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
#[serde(tag = "command")]
pub enum NodeCommand {
  ConsoleLog {params: ConsoleLogParam},
  GetProcessInfo,
  GetStartArgs,
  GetDataBlob {id: String},
  ExecuteSync {script: String},
  ExecuteSyncResponse {uuid: String, data:Option<String>, error:Option<String>},
  Addon {data: String},
  Api {data: String}
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