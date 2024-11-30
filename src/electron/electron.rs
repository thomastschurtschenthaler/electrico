use std::{path::{Path, PathBuf}, time::Duration};
use log::{debug, trace};
use muda::Menu;
use reqwest::StatusCode;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tokio::{runtime::Runtime, time::sleep};
use wry::RequestAsyncResponder;

use crate::{backend::Backend, common::{check_and_create_dir, respond_404, respond_ok, respond_status, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, electron::types::BrowserWindowDevToolsCall, frontend::Frontend, node::node::AppEnv, types::{ElectricoEvents, Package}};
use super::{apis::apis::process_command, menu::create_menu, types::{BrowserWindowBoundsAction, BrowserWindowMaximizedAction, BrowserWindowMinimizedAction, ElectronCommand}};

pub fn process_electron_command(tokio_runtime:&Runtime, event_loop:&EventLoopWindowTarget<ElectricoEvents>, proxy:EventLoopProxy<ElectricoEvents>,
    app_env:&mut AppEnv, rsrc_dir:&PathBuf, package:&Package,
    frontend:&mut Frontend, backend:&mut Backend, command:ElectronCommand,responder:RequestAsyncResponder, data_blob:Option<Vec<u8>>) -> Option<Menu> {
    let mut menu_ret: Option<Menu> = None;
    match command {
        ElectronCommand::BrowserWindowCreate { params } => {
            trace!("BrowserWindowCreate {}", params.id);
            frontend.create_window(params.id.clone(), event_loop, proxy, params);
            respond_ok(responder);
        },
        ElectronCommand::BrowserWindowLoadfile { params } => {
            trace!("BrowserWindowLoadFile {} {}", params.id, params.file);
            frontend.load_url(&params.id, params.file);
            backend.command_callback("BrowserWindowLoadfile".to_string(), params.id);
            respond_ok(responder);
        },
        ElectronCommand::BrowserWindowSetTitle { id, title } => {
            frontend.set_title(&id, title);
            respond_ok(responder);
        },
        ElectronCommand::BrowserWindowGetTitle { id } => {
            if let Some(title) = frontend.get_title(&id) {
                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), title.into_bytes(), responder);
            } else {
                respond_404(responder);
            }
        }
        ElectronCommand::BrowserWindowShow { id , shown} => {
            frontend.show(&id, shown);
            respond_ok(responder);
        },
        ElectronCommand::BrowserWindowBounds { id, params} => {
            match params {
                BrowserWindowBoundsAction::Get => {
                    let bounds = frontend.content_bounds(&id);
                    if let Some(bounds) = bounds {
                        match serde_json::to_string(&bounds) {
                            Ok(json) => {
                                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                            },
                            Err(e) => {
                                respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("BrowserWindowBoundsAction::Get json serialization error: {}", e).into_bytes(), responder);
                            }
                        }
                    } else {
                        respond_404(responder);
                    }
                },
                BrowserWindowBoundsAction::Set {bounds} => {
                    frontend.set_content_bounds(&id, bounds);
                    respond_ok(responder);
                }
            }
        },
        ElectronCommand::BrowserWindowMaximized { id, params} => {
            match params {
                BrowserWindowMaximizedAction::Get => {
                    let maximized = frontend.is_maximized(&id);
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), maximized.to_string().into_bytes(), responder);       
                },
                BrowserWindowMaximizedAction::Set {maximized} => {
                    frontend.set_maximized(&id, maximized);
                    respond_ok(responder);
                }
            }
        },
        ElectronCommand::BrowserWindowMinimized { id, params} => {
            match params {
                BrowserWindowMinimizedAction::Get => {
                    let minimized = frontend.is_minimized(&id);
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), minimized.to_string().into_bytes(), responder);       
                },
                BrowserWindowMinimizedAction::Set {minimized} => {
                    frontend.set_minimized(&id, minimized);
                    respond_ok(responder);
                }
            }
        },
        ElectronCommand::BrowserWindowClose { id} => {
            frontend.close(event_loop, &id);
            if frontend.count()==0 {
                backend.window_all_closed();
            }
            respond_ok(responder);
        },
        ElectronCommand::ChannelSendMessage {id, rid, channel, args} => {
            frontend.send_channel_message(proxy, id, rid, channel, args, data_blob);
            respond_ok(responder);
        },
        ElectronCommand::ExecuteJavascript {id, script} => {
            frontend.execute_javascript(&id, &script);
            respond_ok(responder);
        },
        ElectronCommand::BrowserWindowDevTools { params } => {
            trace!("BrowserWindowDevTools {}", params.id);
            match params.call {
                BrowserWindowDevToolsCall::Open => {
                    frontend.open_devtools(&params.id);
                },
                BrowserWindowDevToolsCall::Close => {
                    frontend.close_devtools(&params.id);
                }
            }
            respond_ok(responder);
        },
        ElectronCommand::AppQuit {exit} => {
            respond_ok(responder);
            tokio_runtime.spawn(
                async move {
                    sleep(Duration::from_millis(100)).await;
                    let _ = proxy.send_event(ElectricoEvents::Exit);
                }
            );
        },
        ElectronCommand::AppSetName { name } => {
            app_env.app_name = Some(name);
            respond_ok(responder);
        },
        ElectronCommand::SetApplicationMenu { menu } => {
            if let Some(menu) = menu {
                menu_ret = Some(create_menu(frontend.get_actual_window(), menu, &app_env.app_name));
            }
            respond_ok(responder);
        },
        ElectronCommand::GetAppPath { path } => {
            if let Some(path) = path {
                if path=="appData" {
                    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", package.name.as_str()) {
                        let dir = proj_dirs.config_dir().as_os_str().to_str().unwrap().to_string();
                        check_and_create_dir(Path::new(&dir));
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), dir.into_bytes(), responder);
                    } else {
                        respond_404(responder);
                    }
                } else if path=="logs" {
                    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", package.name.as_str()) {
                        let dir = proj_dirs.cache_dir().as_os_str().to_str().unwrap().to_string();
                        check_and_create_dir(Path::new(&dir));
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), dir.into_bytes(), responder);
                    } else {
                        respond_404(responder);
                    }
                } else if path=="temp" {
                    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", package.name.as_str()) {
                        let dir = proj_dirs.cache_dir().as_os_str().to_str().unwrap().to_string();
                        check_and_create_dir(Path::new(&dir));
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), dir.into_bytes(), responder);
                    } else {
                        respond_404(responder);
                    }
                } else if path=="userData" {
                    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", package.name.as_str()) {
                        let dir = proj_dirs.config_dir().as_os_str().to_str().unwrap().to_string();
                        check_and_create_dir(Path::new(&dir));
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), dir.into_bytes(), responder);
                    } else {
                        respond_404(responder);
                    }
                } else if path=="userHome" {
                    if let Some(user_dirs) = directories::UserDirs::new() {
                        let dir = user_dirs.home_dir().as_os_str().to_str().unwrap().to_string();
                        respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), dir.into_bytes(), responder);
                    } else {
                        respond_404(responder);
                    }
                }
            } else {
                respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), rsrc_dir.as_os_str().to_str().unwrap().to_string().into_bytes(), responder);
            }
        },
        ElectronCommand::GetAppVersion => {
            respond_status(StatusCode::OK, CONTENT_TYPE_TEXT.to_string(), package.version.clone().into_bytes(), responder);
        },
        ElectronCommand::GetPrimaryDisplay => {
            respond_ok(responder);
        },
        ElectronCommand::ShellOpenExternal { url } => {
            let _  = open::that(url);
            respond_ok(responder);
        },
        ElectronCommand::PrintToPDF { id } => {
            // TODO - silent printing
            frontend.print(&id);
            tokio_runtime.spawn(
                async move {
                    sleep(Duration::from_secs(10)).await;
                    respond_ok(responder);
                }
            );
        },
        ElectronCommand::RegisterFileProtocol { schema } => {
            frontend.register_file_protocol(schema);
            respond_ok(responder);
        },
        ElectronCommand::Api { data } => {
            process_command(tokio_runtime, app_env, proxy, backend, frontend, data, responder, data_blob);
        }
    }
    menu_ret
}
