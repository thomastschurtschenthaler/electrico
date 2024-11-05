use log::{debug, error};
use serde_json::Error;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::respond_404, node::node::AppEnv, types::ElectricoEvents};

use super::{pty::process_pty_command, spdlog::process_spdlog_command, sqlite::process_sqllite_command, types::AddonCommand};

pub fn process_command(tokio_runtime:&Runtime, app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    data:String,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    let command:Result<AddonCommand, Error> = serde_json::from_str(data.as_str());
    match command {
        Ok(command) => {
            match command {
                AddonCommand::SQLite { command } => {
                    process_sqllite_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                },
                AddonCommand::SPDLog { command } => {
                    process_spdlog_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                },
                AddonCommand::PTY { command } => {
                    process_pty_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                }
            }
        },
        Err(e) => {
            error!("addon process_command serde error {}", e);
            respond_404(responder);
        }

    }
}