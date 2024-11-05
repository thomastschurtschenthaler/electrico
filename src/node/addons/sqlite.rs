use std::collections::HashMap;

use log::debug;
use reqwest::StatusCode;
use rusqlite::{params_from_iter, Connection};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use uuid::Uuid;
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::{respond_ok, respond_status, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, node::node::AppEnv, types::ElectricoEvents};

use super::types::SQLiteCommand;

pub fn process_sqllite_command(_tokio_runtime:&Runtime, _app_env:&AppEnv,
    _proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:SQLiteCommand,
    responder:RequestAsyncResponder,
    _data_blob:Option<Vec<u8>>)  {
    
    match command {
        SQLiteCommand::Connect { path } => {
            match Connection::open(path) {
                Ok(c) => {
                    let id = Uuid::new_v4().to_string();
                    backend.addon_state_insert(&id, c);
                    respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), id.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                }
            }
        },
        SQLiteCommand::Close {cid } => {
            backend.addon_state_remove(&cid);
            respond_ok(responder);
        }
        SQLiteCommand::Exec { cid, sql , params} => {
            let c:Option<&mut Connection> = backend.addon_state_get_mut(&cid);
            if let Some(c) = c {
                if let Some(params) = params {
                    match c.execute(&sql, params_from_iter(params.iter())) {
                        Ok(_) => {
                            respond_ok(responder);
                        },
                        Err(e) => {
                            respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                        }
                    }
                    return;
                }
                match c.execute_batch(&sql) {
                    Ok(_) => {
                        respond_ok(responder);
                    },
                    Err(e) => {
                        respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                    }
                }
            } else {
                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: No Connection for cid {}", cid).into_bytes(), responder);
            }
        },
        SQLiteCommand::Serialize { cid } => {
            let c:Option<&mut Connection> = backend.addon_state_get_mut(&cid);
            if let Some(_c) = c {
                debug!("SQLiteSerialize OK");
                respond_ok(responder);
            } else {
                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: No Connection for cid {}", cid).into_bytes(), responder);
            }
        },
        SQLiteCommand::Run { cid, cmd } => {
            let c:Option<&mut Connection> = backend.addon_state_get_mut(&cid);
            if let Some(c) = c {
                if cmd=="BEGIN TRANSACTION" {
                    match c.execute_batch("BEGIN TRANSACTION") {
                        Err(e) => {
                            respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                            return;
                        },
                        _=>{}
                    }
                }
                if cmd=="END TRANSACTION" {
                    match c.execute_batch("COMMIT") { 
                        Err(e) => {
                            respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                            return;
                        },
                        _=>{
                        }
                    }
                }
                respond_ok(responder);
            } else {
                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: No Connection for cid {}", cid).into_bytes(), responder);
            }
        },
        SQLiteCommand::Query { cid, sql, all } => {
            let c:Option<&mut Connection> = backend.addon_state_get_mut(&cid);
            if let Some(c) = c {
                match c.prepare(&sql) {
                    Ok(mut s) => {
                        let mut res:Vec<Vec<String>> = Vec::new();
                        match s.query([]) {
                            Ok(mut r) => {
                                while let Some(row) = r.next().unwrap() {
                                    let mut ix:usize=0;
                                    let mut res_row:Vec<String> = Vec::new();
                                    loop {
                                        match row.get(ix) {
                                            Ok(col) => {
                                                let value:String = col;
                                                res_row.push(value);
                                                ix+=1;
                                            },
                                            Err(_e) => {
                                                break;
                                            }
                                        }
                                    }
                                    res.push(res_row);
                                    if !all {
                                        break;
                                    }
                                }  
                            },
                            Err(e) => {
                                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                                return;
                            }
                        }
                        let mut mres:Vec<HashMap<String, String>> = Vec::new();
                        for r in res {
                            let mut mres_row:HashMap<String, String> = HashMap::new();
                            let mut ix = 0;
                            for v in r {
                                mres_row.insert(s.column_name(ix).unwrap().to_string(), v);
                                ix+=1;
                            }
                            mres.push(mres_row);
                        };
                        match serde_json::to_string(&mres) {
                            Ok(json) => {
                                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                            },
                            Err(e) => {
                                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("SQLiteQuery json serialization error: {}", e).into_bytes(), responder);
                            }
                        }
                    },
                    Err(e) => {
                        respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: {}", e).into_bytes(), responder);
                    }
                }
            } else {
                respond_status(StatusCode::BAD_REQUEST, CONTENT_TYPE_TEXT.to_string(), format!("Error: No Connection for cid {}", cid).into_bytes(), responder);
            }
        }
    }
}