
use log::{error, debug, trace};
use reqwest::StatusCode;
use rfd::{MessageButtons, MessageLevel};
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;
use wry::{http::Response, RequestAsyncResponder};

use crate::{backend::Backend, common::{respond_404, respond_ok, respond_status, CONTENT_TYPE_BIN, CONTENT_TYPE_JSON, CONTENT_TYPE_TEXT}, frontend::Frontend, node::{apis::types::FSDirent, common::send_command, node::AppEnv}, types::{BackendCommand, ElectricoEvents}};

use super::types::DialogCommand;

pub fn process_dialog_command(tokio_runtime:&Runtime, _app_env:&AppEnv,
    proxy:EventLoopProxy<ElectricoEvents>,
    backend:&mut Backend,
    frontend:&mut Frontend,
    command:DialogCommand,
    responder:RequestAsyncResponder,
    data_blob:Option<Vec<u8>>)  {
    
    match command {
        DialogCommand::ShowOpenDialogSync { options } => {
            let mut picked:Vec<String>=Vec::new();
            if let Some(properties) = options.properties {
                let mut fd = rfd::FileDialog::new();
                if let Some(filters) = options.filters {
                    for filter in filters {
                        let ext:Vec<&str> = filter.extensions.iter().map(|e| &**e).collect();
                        fd = fd.add_filter(filter.name, &ext);
                    }
                }
                if let Some(title) = options.title {
                    fd = fd.set_title(title);
                }
                if let Some(default_path) = options.default_path {
                    fd = fd.set_directory(default_path);
                }
                if properties.contains(&"createDirectory".to_string()) {
                    fd = fd.set_can_create_directories(true);
                }
                if properties.contains(&"openDirectory".to_string()) {
                    if properties.contains(&"multiSelections".to_string()) {
                        match fd.pick_folders() {
                            Some(sel) => {
                                for p in sel {
                                    picked.push(p.as_os_str().to_str().unwrap().to_string());
                                }
                            },
                            None => {}
                        }
                    } else {
                        match fd.pick_folder() {
                            Some(sel) => {
                                picked.push(sel.as_os_str().to_str().unwrap().to_string());
                            },
                            None => {}
                        }
                    }
                } else if properties.contains(&"openFile".to_string()) {
                    if properties.contains(&"multiSelections".to_string()) {
                        match fd.pick_files() {
                            Some(sel) => {
                                for p in sel {
                                    picked.push(p.as_os_str().to_str().unwrap().to_string());
                                }
                            },
                            None => {}
                        }
                    } else {
                        match fd.pick_file() {
                            Some(sel) => {
                                picked.push(sel.as_os_str().to_str().unwrap().to_string());
                            },
                            None => {}
                        }
                    }
                } 
            }
            match serde_json::to_string(&picked) {
                Ok(json) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("ShowOpenDialog json serialization error: {}", e).into_bytes(), responder);
                }
            }
        },
        DialogCommand::ShowOpenDialog { window_id, options } => {
            if let Some(window_id) = window_id {
                frontend.set_focus(&window_id);
            }
            tokio_runtime.spawn(
                async move {
                    let mut picked:Vec<String>=Vec::new();
                    if let Some(properties) = options.properties {
                        let mut fd = rfd::AsyncFileDialog::new();
                        if let Some(filters) = options.filters {
                            for filter in filters {
                                let ext:Vec<&str> = filter.extensions.iter().map(|e| &**e).collect();
                                fd = fd.add_filter(filter.name, &ext);
                            }
                        }
                        if let Some(title) = options.title {
                            fd = fd.set_title(title);
                        }
                        if let Some(default_path) = options.default_path {
                            fd = fd.set_directory(default_path);
                        }
                        if properties.contains(&"createDirectory".to_string()) {
                            fd = fd.set_can_create_directories(true);
                        }
                        if properties.contains(&"openDirectory".to_string()) {
                            if properties.contains(&"multiSelections".to_string()) {
                                match fd.pick_folders().await {
                                    Some(sel) => {
                                        for p in sel {
                                            picked.push(p.path().as_os_str().to_str().unwrap().to_string());
                                        }
                                    },
                                    None => {}
                                }
                            } else {
                                match fd.pick_folder().await {
                                    Some(sel) => {
                                        picked.push(sel.path().as_os_str().to_str().unwrap().to_string());
                                    },
                                    None => {}
                                }
                            }
                        } else if properties.contains(&"openFile".to_string()) {
                            if properties.contains(&"multiSelections".to_string()) {
                                match fd.pick_files().await {
                                    Some(sel) => {
                                        for p in sel {
                                            picked.push(p.path().as_os_str().to_str().unwrap().to_string());
                                        }
                                    },
                                    None => {}
                                }
                            } else {
                                match fd.pick_file().await {
                                    Some(sel) => {
                                        let mut p:Vec<String> = Vec::new();
                                        picked.push(sel.path().as_os_str().to_str().unwrap().to_string());
                                    },
                                    None => {}
                                }
                            }
                        }
                    }
                    match serde_json::to_string(&picked) {
                        Ok(json) => {
                            respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                        },
                        Err(e) => {
                            respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("ShowOpenDialog json serialization error: {}", e).into_bytes(), responder);
                        }
                    }
                }
            );
        },
        DialogCommand::ShowSaveDialogSync { options } => {
            let mut picked:Option<String>=None;
            if let Some(properties) = options.properties {
                let mut fd = rfd::FileDialog::new();
                if let Some(filters) = options.filters {
                    for filter in filters {
                        let ext:Vec<&str> = filter.extensions.iter().map(|e| &**e).collect();
                        fd = fd.add_filter(filter.name, &ext);
                    }
                }
                if let Some(title) = options.title {
                    fd = fd.set_title(title);
                }
                if let Some(default_path) = options.default_path {
                    fd = fd.set_directory(default_path);
                }
                if properties.contains(&"createDirectory".to_string()) {
                    fd = fd.set_can_create_directories(true);
                }
                match fd.save_file() {
                    Some(sel) => {
                        picked = Some(sel.as_os_str().to_str().unwrap().to_string());
                    },
                    None => {}
                }
            }
            match serde_json::to_string(&picked) {
                Ok(json) => {
                    respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), json.into_bytes(), responder);
                },
                Err(e) => {
                    respond_status(StatusCode::INTERNAL_SERVER_ERROR, CONTENT_TYPE_TEXT.to_string(), format!("ShowOpenDialog json serialization error: {}", e).into_bytes(), responder);
                }
            }
        },
        DialogCommand::ShowSaveDialog { window_id, options } => {
            if let Some(window_id) = window_id {
                frontend.set_focus(&window_id);
            }
            tokio_runtime.spawn(
                async move {
                    let mut fd = rfd::AsyncFileDialog::new();
                    if let Some(filters) = options.filters {
                        for filter in filters {
                            let ext:Vec<&str> = filter.extensions.iter().map(|e| &**e).collect();
                            fd = fd.add_filter(filter.name, &ext);
                        }
                    }
                    if let Some(title) = options.title {
                        fd = fd.set_title(title);
                    }
                    if let Some(default_path) = options.default_path {
                        fd = fd.set_directory(default_path);
                    }
                    if let Some(properties) = options.properties {
                    if properties.contains(&"createDirectory".to_string()) {
                            fd = fd.set_can_create_directories(true);
                        }
                    }
                    match fd.save_file().await {
                        Some(sel) => {
                            let picked = sel.path().as_os_str().to_str().unwrap().to_string();
                            respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), picked.into_bytes(), responder);
                        },
                        None => {
                            respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), "".to_string().into_bytes(), responder);
                        }
                    }
                }
            );
        },
        DialogCommand::ShowMessageBoxSync { options } => {
            let mut dialog = rfd::MessageDialog::new();
            let mut desc = options.message;
            if let Some(detail) = options.detail {
                desc = desc + " - " + detail.as_str();
            }
            dialog = dialog.set_description(desc);
            if let Some(title) = options.title {
                dialog = dialog.set_title(title.as_str());
            }
            if let Some(msg_type) = options.msg_type {
               if msg_type=="error" {
                   dialog = dialog.set_level(MessageLevel::Error);
               } else if msg_type=="info" {
                   dialog = dialog.set_level(MessageLevel::Info);
               }
            }
            respond_ok(responder);
            dialog.show();
            
        },
        DialogCommand::ShowMessageBox {window_id, options } => {
            if let Some(window_id) = window_id {
                frontend.set_focus(&window_id);
            }
            tokio_runtime.spawn(
                async move {
                    let mut dialog = rfd::AsyncMessageDialog::new();
                    let mut desc = options.message;
                    if let Some(detail) = options.detail {
                        desc = desc + " - " + detail.as_str();
                    }
                    dialog = dialog.set_description(desc);
                    let mut ok = "Ok".to_string();
                    let mut cancel = "Cancel".to_string();
                    if let Some(buttons) = options.buttons {
                        if buttons.len()>0 {
                            let btns:MessageButtons;
                            ok = buttons.get(0).unwrap().clone();
                            if buttons.len()==1 {
                                btns=MessageButtons::OkCustom(ok.clone());
                            } else {
                                cancel = buttons.get(1).unwrap().clone();
                                if buttons.len()==2 {
                                    btns=MessageButtons::OkCancelCustom(ok.clone(), cancel.clone());
                                } else {
                                    btns=MessageButtons::YesNoCancelCustom(ok.clone(), cancel.clone(), buttons.get(2).unwrap().clone());
                                }
                            }
                            dialog = dialog.set_buttons(btns);
                        }
                    }
                   
                    if let Some(title) = options.title {
                        dialog = dialog.set_title(title.as_str());
                    }
                    if let Some(msg_type) = options.msg_type {
                        if msg_type=="error" {
                            dialog = dialog.set_level(MessageLevel::Error);
                        } else if msg_type=="info" {
                            dialog = dialog.set_level(MessageLevel::Info);
                        }
                    }
                    let res = dialog.show().await;
                    match res {
                        rfd::MessageDialogResult::Custom(custom) => {
                            if custom==ok {
                                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), "0".to_string().into_bytes(), responder);
                            } else if custom==cancel {
                                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), "1".to_string().into_bytes(), responder);
                            } else {
                                respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), "2".to_string().into_bytes(), responder);
                            }
                        },
                        _ => {
                            respond_status(StatusCode::OK, CONTENT_TYPE_JSON.to_string(), "".to_string().into_bytes(), responder);
                        }
                    }
                    
                }
            );
        }
    }
}