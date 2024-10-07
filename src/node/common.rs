use tao::event_loop::EventLoopProxy;
use std::sync::mpsc::Sender;

use crate::types::{BackendCommand, ElectricoEvents};

pub fn send_command(proxy:&EventLoopProxy<ElectricoEvents>, command_sender:&Sender<BackendCommand>, command:BackendCommand) {
    let _ = command_sender.send(command);
    let _ = proxy.send_event(ElectricoEvents::Noop);
}