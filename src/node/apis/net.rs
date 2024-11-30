use log::{error, debug, trace};
use reqwest::StatusCode;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::{respond_ok, respond_status, CONTENT_TYPE_TEXT}, node::{apis::ipc::{ipc_connection, ipc_server}, node::AppEnv}, types::ElectricoEvents};

use super::types::NETCommand;

pub fn process_net_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:NETCommand,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    
    let command_sender = backend.command_sender();
    match command {
        NETCommand::CreateServer {hook, options } => {
            trace!("NETCreateServer {}", hook);
            ipc_server(hook, tokio_runtime, proxy, command_sender, responder);
        },
        NETCommand::CloseServer { id } => {
            backend.net_server_close(id);
            respond_ok(responder);
        },
        NETCommand::CloseConnection { id } => {
            backend.net_connection_close(id);
            respond_ok(responder);
        },
        NETCommand::CreateConnection { hook, id } => {
            trace!("NETCreateConnection {}, {}", hook, id);
            ipc_connection(hook, id, tokio_runtime, proxy, command_sender, responder);
        },
        NETCommand::WriteConnection { id } => {
            trace!("NETWriteConnection {}", id);
            if let Some(data) = data_blob {
                backend.net_write_connection(id, data);
                respond_ok(responder);
            } else {
                error!("NETWriteConnection error, no data");
                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("NETWriteConnection error, no data").into_bytes(), responder);
            }
        },
        NETCommand::SetTimeout { id, timeout } => {
            trace!("NETSetTimeout {}, {}", id, timeout);
            backend.net_set_timeout(id, timeout);
            respond_ok(responder);
        }
    }
}