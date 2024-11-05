use std::sync::Arc;
use spdlog::{critical, debug, error, info, sink::{FileSink, Sink}, trace, warn, Logger};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::RequestAsyncResponder;
use crate::{backend::Backend, common::{respond_404, respond_ok}, node::node::AppEnv, types::ElectricoEvents};
use super::types::SPDLogCommand;

pub fn process_spdlog_command(_tokio_runtime:&Runtime, _app_env:&AppEnv,
    _proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    command:SPDLogCommand,
    responder:RequestAsyncResponder,
    _data_blob:Option<Vec<u8>>)  {
    
    match command {
        SPDLogCommand::CreateLogger { id, name, filepath } => {
            match FileSink::builder().path(filepath).build() {
                Ok(sink) => {
                    match Logger::builder().name(name).sink(Arc::new(sink)).build() {
                        Ok(logger) => {
                            backend.addon_state_insert(&id, logger);
                            respond_ok(responder);
                        },
                        Err(e) => {
                            log::error!("CreateLogger Error: {}", e);
                            respond_404(responder);
                        }
                    }
                },
                Err(e) => {
                    log::error!("CreateLogger Build FileSink Error: {}", e);
                    respond_404(responder);
                }
            }
        },
        SPDLogCommand::SetLogLevel { id, level } => {
            let logger:Option<&mut Logger> = backend.addon_state_get_mut(&id);
            if let Some(logger) = logger {
                match level {
                    super::types::SPDLogLevel::Trace => {
                        logger.set_level_filter(spdlog::LevelFilter::All);
                    },
                    crate::node::addons::types::SPDLogLevel::Debug => {
                        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(spdlog::Level::Debug));
                    },
                    crate::node::addons::types::SPDLogLevel::Error => {
                        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(spdlog::Level::Error));
                    },
                    crate::node::addons::types::SPDLogLevel::Info => {
                        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(spdlog::Level::Info));
                    },
                    crate::node::addons::types::SPDLogLevel::Warn => {
                        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(spdlog::Level::Warn));
                    },
                    crate::node::addons::types::SPDLogLevel::Critical => {
                        logger.set_level_filter(spdlog::LevelFilter::MoreSevereEqual(spdlog::Level::Critical));
                    },
                    crate::node::addons::types::SPDLogLevel::Off => {
                        logger.set_level_filter(spdlog::LevelFilter::Off);
                    }
                }
                respond_ok(responder);
            } else {
                log::error!("SPDLogger not found: {}", id);
                respond_404(responder);
            }
        },
        SPDLogCommand::Log { id, level, message } => {   
            let logger:Option<&mut Logger> = backend.addon_state_get_mut(&id);
            if let Some(logger) = logger {
                match level {
                    super::types::SPDLogLevel::Trace => {
                        trace!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Debug => {
                        debug!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Error => {
                        error!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Info => {
                        info!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Warn => {
                        warn!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Critical => {
                        critical!(logger:logger, "{}", message);
                    },
                    crate::node::addons::types::SPDLogLevel::Off => {
                        
                    }
                }
                logger.flush();
                respond_ok(responder);
            } else {
                log::error!("SPDLogger not found: {}", id);
                respond_404(responder);
            }
        },
        SPDLogCommand::Flush { id } => {
            let logger:Option<&mut Logger> = backend.addon_state_get_mut(&id);
            if let Some(logger) = logger {
                logger.flush();
                respond_ok(responder);
            } else {
                log::error!("SPDLogger not found: {}", id);
                respond_404(responder);
            }
        }
    }
}