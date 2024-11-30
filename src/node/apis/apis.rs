use log::{debug, error};
use serde_json::Error;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::respond_404, node::node::AppEnv, types::ElectricoEvents};

use super::{child_process::process_childprocess_command, fs::process_fs_command, http::process_http_command, net::process_net_command, types::APICommand};

pub fn process_command(tokio_runtime:&Runtime, app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    data:String,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    let command:Result<APICommand, Error> = serde_json::from_str(data.as_str());
    match command {
        Ok(command) => {
            match command {
                APICommand::FS { command } => {
                    process_fs_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                },
                APICommand::NET { command } => {
                    process_net_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                },
                APICommand::Childprocess { command } => {
                    process_childprocess_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                },
                APICommand::HTTP { command } => {
                    process_http_command(tokio_runtime, app_env, proxy, backend, command, responder, data_blob);
                }
            }
        },
        Err(e) => {
            error!("node apis process_command serde error {}", e);
            respond_404(responder);
        }

    }
}