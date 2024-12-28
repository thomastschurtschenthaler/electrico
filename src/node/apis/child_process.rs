use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;

use crate::{backend::Backend, common::respond_ok, node::node::AppEnv, types::{ElectricoEvents, Responder}};

use super::{process::child_process_spawn, types::ChildprocessCommand};

pub fn process_childprocess_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:ChildprocessCommand,
    responder:Responder,
    data_blob:Option<Vec<u8>>)  {
    
    let command_sender = backend.command_sender();
    match command {
        ChildprocessCommand::Spawn { cmd, args, options} => {
            child_process_spawn(cmd, args, options, backend, tokio_runtime, proxy, command_sender, responder);
        },
        ChildprocessCommand::StdinWrite { pid , end} => {
            backend.child_process_callback(pid, "stdin".to_string(), end, data_blob);
            respond_ok(responder);
        },
        ChildprocessCommand::Disconnect { pid } => {
            backend.child_process_disconnect(pid);
            respond_ok(responder);
        },
        ChildprocessCommand::Kill { pid  } => {
            backend.child_process_kill(pid);
            respond_ok(responder);
        }
    }
}