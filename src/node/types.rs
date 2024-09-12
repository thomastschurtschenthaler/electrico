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
  pub recursive: Option<bool>
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
  pub is_directory: bool
}
impl FSStat {
  pub fn new(is_directory: bool) -> FSStat {
    FSStat { is_directory }
  }
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
  FSWriteFile {path:String, data:String, options:Option<FSOptions>},
  HTTPRequest {options:HTTPOptions},
  ChildProcessSpawn {cmd: String, args:Option<Vec<String>>},
  ChildProcessStdinWrite {pid: String, data:String}
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