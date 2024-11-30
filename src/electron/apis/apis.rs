use log::{debug, error};
use serde_json::Error;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::respond_404, frontend::Frontend, node::node::AppEnv, types::ElectricoEvents};

use super::{dialog::process_dialog_command, types::APICommand};

pub fn process_command(tokio_runtime:&Runtime, app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    frontend:&mut Frontend,
    data:String,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    let command:Result<APICommand, Error> = serde_json::from_str(data.as_str());
    match command {
        Ok(command) => {
            match command {
                APICommand::Dialog { command } => {
                    process_dialog_command(tokio_runtime, app_env, proxy, backend, frontend, command, responder, data_blob);
                }
            }
        },
        Err(e) => {
            error!("node apis process_command serde error {}", e);
            respond_404(responder);
        }

    }
}