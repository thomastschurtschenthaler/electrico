use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "api")]
pub enum AddonCommand {
    SQLite {command: SQLiteCommand},
    SPDLog {command: SPDLogCommand},
    PTY {command: PTYCommand}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum SQLiteCommand {
    Connect {path: String},
    Close {cid: String},
    Exec {cid: String, sql: String, params:Option<Vec<String>>},
    Serialize {cid: String},
    Run {cid: String, cmd: String},
    Query {cid: String, sql: String, all:bool},
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum SPDLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
    Off
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum SPDLogCommand {
    CreateLogger {id: String, name:String, filepath:String},
    Log {id: String, level:SPDLogLevel, message:String},
    SetLogLevel {id: String, level:SPDLogLevel},
    Flush {id: String}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct  PTYCommandOpt {
    pub name:String,
    pub cols:u16,
    pub rows:u16,
    pub cwd:String,
    pub env:HashMap<String, String>
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum PTYCommand {
    Spawn {id: String, shell:String, args:Vec<String>, opt:PTYCommandOpt}
}