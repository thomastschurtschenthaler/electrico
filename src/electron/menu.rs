use std::{collections::HashMap, sync::Mutex};

use lazy_static::lazy_static;
use log::debug;
use muda::{accelerator::{Accelerator, Code, Modifiers}, Menu, MenuItem, PredefinedMenuItem, Submenu};
use substring::Substring;
use tao::window::Window;

use crate::electron::types::AppMenu;

lazy_static! {
    static ref MODIFIERS: Mutex<HashMap<&'static str, Modifiers>> = {
        let mut m:HashMap<&str, Modifiers> = HashMap::new();
        m.insert("Cmd", Modifiers::SUPER);
        m.insert("Ctrl", Modifiers::CONTROL);
        m.insert("Alt", Modifiers::ALT);
        m.insert("Shift", Modifiers::SHIFT);
        #[cfg(target_os = "macos")]
        m.insert("CmdOrCtrl", Modifiers::SUPER);
        #[cfg(target_os = "windows")]
        m.insert("CmdOrCtrl", Modifiers::CONTROL);
        Mutex::new(m)
    };
    static ref KEYS: Mutex<HashMap<&'static str, Code>> = {
        let mut k:HashMap<&str, Code> = HashMap::new();
        k.insert("A", Code::KeyA);
        k.insert("B", Code::KeyB);
        k.insert("C", Code::KeyC);
        k.insert("D", Code::KeyD);
        k.insert("E", Code::KeyE);
        k.insert("F", Code::KeyF);
        k.insert("G", Code::KeyG);
        k.insert("H", Code::KeyH);
        k.insert("I", Code::KeyI);
        k.insert("J", Code::KeyJ);
        k.insert("K", Code::KeyK);
        k.insert("L", Code::KeyL);
        k.insert("M", Code::KeyM);
        k.insert("N", Code::KeyN);
        k.insert("O", Code::KeyO);
        k.insert("P", Code::KeyP);
        k.insert("Q", Code::KeyQ);
        k.insert("R", Code::KeyR);
        k.insert("S", Code::KeyS);
        k.insert("T", Code::KeyT);
        k.insert("U", Code::KeyU);
        k.insert("V", Code::KeyV);
        k.insert("W", Code::KeyW);
        k.insert("X", Code::KeyX);
        k.insert("Y", Code::KeyY);
        k.insert("Z", Code::KeyZ);
        k.insert("-", Code::Minus);
        k.insert("+", Code::NumpadAdd);
        k.insert("=", Code::Equal);
        Mutex::new(k)
    };    
}

fn populate_menu(sub_menu:&Submenu, menu:&Vec<super::types::Menu>, app_name:&Option<String>, keys_map:&std::sync::MutexGuard<'_, HashMap<&str, Code>>, mods_map:&std::sync::MutexGuard<'_, HashMap<&str, Modifiers>>) {
    for item in menu.iter() {
        let mut accelerator:Option<Accelerator> = None;
        if let Some(acc) = item.accelerator.clone() {
            let keystr:&str;
            let mut acc_modifier:Option<Modifiers> = None;
            if let Some(pos) = acc.rfind("+") {
                keystr = acc.substring(pos+1, acc.len());
                let modstr = acc.substring(0, pos);
                let mut modifier = Modifiers::empty();
                for modpart in modstr.split("+") {
                    if let Some(modif) = mods_map.get(modpart) {
                        modifier = modifier | *modif;
                    }
                }
                acc_modifier = Some(modifier);
            } else {
                keystr = acc.as_str();
            }
            if let Some(key) = keys_map.get(keystr) {
                accelerator = Some(Accelerator::new(acc_modifier, *key));
            }
        }
        if let Some(label) = item.label.clone() {
            if let Some(submenu) = &item.submenu {
                let sub_sub_menu = Submenu::new(label, true);
                populate_menu(&sub_sub_menu, submenu, app_name, keys_map, mods_map);
                let _ = sub_menu.append(&sub_sub_menu);
            } else {
                let _ = sub_menu.append(
                    &MenuItem::with_id(item.id.as_str(), label, true, accelerator)
                );
            }
        } else if let Some(item_type) = item.item_type.clone() {
            if item_type=="separator" {
            let _ = sub_menu.append(&PredefinedMenuItem::separator());
            }
        } else if let Some(role) = item.role.clone() {
            if role=="quit" {
                let mut label = "Quit".to_string();
                if let Some(app_name) = app_name {
                    label = label + " " + app_name.as_str();
                }
                let _ = sub_menu.append(
                    &MenuItem::with_id("quit", label.as_str(), true, accelerator)
                );
            } else if role=="cut" {
                let _ = sub_menu.append(&PredefinedMenuItem::cut(None));
            } else if role=="copy" {
                let _ = sub_menu.append(&PredefinedMenuItem::copy(None));
            } else if role=="paste" {
                let _ = sub_menu.append(&PredefinedMenuItem::paste(None));
            } else if role=="undo" {
                let _ = sub_menu.append(&PredefinedMenuItem::undo(None));
            } else if role=="redo" {
                let _ = sub_menu.append(&PredefinedMenuItem::redo(None));
            } else if role=="hide" {
                let _ = sub_menu.append(&PredefinedMenuItem::hide(None));
            } else if role=="hideothers" {
                let _ = sub_menu.append(&PredefinedMenuItem::hide_others(None));
            } else if role=="unhide" {
                let _ = sub_menu.append(&PredefinedMenuItem::bring_all_to_front(None));
            } else if role=="close" {
                let _ = sub_menu.append(&PredefinedMenuItem::close_window(None));
            } else if role=="toggleDevTools" {
                let mut label: String = "Toggle Developer Tools".to_string();
                if let Some(lab) = item.label.clone() {
                    label = lab;
                }
                let _ = sub_menu.append(&MenuItem::with_id("toggleDevTools", label, true, accelerator));
            }
        }
    }
}

pub fn create_menu(window:Option<&Window>, menu:Vec<AppMenu>, app_name:&Option<String>) -> Menu {
    let keys_map: std::sync::MutexGuard<'_, HashMap<&str, Code>> = KEYS.lock().unwrap();
    let mods_map: std::sync::MutexGuard<'_, HashMap<&str, Modifiers>> = MODIFIERS.lock().unwrap();
    let main_menu = Menu::new();
    for app_menu in menu.iter() {
        let mut label = "".to_string();
        if let Some(mlab) = app_menu.label.clone() {
            label = mlab;
        }
        let sub_menu = Submenu::new(label, true);
        populate_menu(&sub_menu, &app_menu.submenu, app_name, &keys_map, &mods_map);
        let _ = main_menu.append(&sub_menu);
    }
    if let Some(window) = window {
        #[cfg(target_os = "windows")] {
            use tao::platform::windows::WindowExtWindows;
            main_menu.init_for_hwnd(window.hwnd() as _).unwrap();
        }
        #[cfg(target_os = "linux")] {
            use tao::platform::unix::WindowExtUnix;
            let _ = main_menu.init_for_gtk_window(window.gtk_window(), window.default_vbox());
        }
    }
    #[cfg(target_os = "macos")]
    main_menu.init_for_nsapp();
    main_menu
}
