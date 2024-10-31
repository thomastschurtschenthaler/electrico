#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "addon")]
pub enum AddonCommand {
    SQLite {command: SQLiteCommand},
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