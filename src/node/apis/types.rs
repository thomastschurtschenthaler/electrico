use std::{collections::HashMap, time::SystemTime};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "api")]
pub enum APICommand {
  FS {command: FSCommand},
  NET {command: NETCommand},
  Childprocess {command: ChildprocessCommand},
  HTTP {command: HTTPCommand},
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum HTTPCommand {
  Request {options:HTTPOptions}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct HTTPOptions {
  pub hostname: String,
  pub port: i32,
  pub path: String,
  pub method: String
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SpawnOptions {
  pub cwd:Option<String>,
  pub env:Option<HashMap<String, String>>
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum ChildprocessCommand {
  Spawn {cmd: Option<String>, args:Option<Vec<String>>, options:Option<SpawnOptions>},
  StdinWrite {pid: String, end:bool},
  Disconnect {pid: String},
  Kill {pid: String}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum NETCommand {
  CreateServer {hook:String, options: Option<NETOptions>},
  CloseServer {id:String},
  CloseConnection {id:String},
  CreateConnection {hook:String, id:String},
  WriteConnection {id:String},
  SetTimeout {id:String, timeout:u128},
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NETOptions {
  
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum FSCommand {
  Access {path:String, mode:i32},
  Lstat {path:String},
  Rm {path:String, options:Option<FSOptions>},
  Mkdir {path:String, options:Option<FSOptions>},
  CopyFile {src:String, dest:String, mode:Option<i32>},
  ReadFile {path:String, options:Option<FSOptions>},
  ReadDir {path:String, options:Option<FSOptions>},
  WriteFile {path:String, options:Option<FSOptions>},
  Watch {path:String, wid:String, options:Option<FSOptions>},
  WatchClose {wid:String},
  Open {fd:i64, path:String, flags:String, mode:String},
  Close {fd:i64},
  Read {fd:i64, offset:i64, length:usize, position:Option<u64>},
  Write {fd:i64, offset:i64, length:usize, position:Option<u64>},
  RealPath {path:String},
  Fdatasync {fd:i64},
  Unlink {path:String},
  Rename {old_path:String, new_path:String},
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FSOptions {
  pub encoding: Option<String>,
  pub recursive: Option<bool>,
  #[serde(rename = "withFileTypes")]
  pub with_file_types:  Option<bool>
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FSStat {
  #[serde(rename = "isDirectory")]
  pub is_directory: bool,
  pub size: Option<u64>,
  pub birthtime: Option<SystemTime>,
  pub mtime: Option<SystemTime>
}
impl FSStat {
  pub fn new(is_directory: bool, size:Option<u64>, birthtime:Option<SystemTime>, mtime:Option<SystemTime>) -> FSStat {
    FSStat { size, is_directory, birthtime, mtime }
  }
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