use crate::core;
use crate::core::config::IConfig;
use crate::core::tray::Tray;
use crate::{ret_err, wrap_err};
use anyhow::{Context, Ok, Result};
use tauri::Icon;
type CmdResult<T = ()> = Result<T, String>;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/* 重启xray */
#[tauri::command]
pub fn restart_xray() {
    crate::log_err!(core::xray::Xray::reload_xray())
}

/* 打开目录 */
#[tauri::command]
pub fn open_app_home_dir() {
    core::path::AppPath::app_home_dir()
        .context("fail get home dir")
        .and_then(|path| {
            open::that(path.clone()).context(format!("fail open path {}", path.display()))
        })
        .map_err(|err| log::error!(target: "app", "[cmd]: {err}"))
        .ok();
}

#[tauri::command]
pub fn open_core_dir() {
    core::path::AppPath::app_core_dir()
        .context("fail get resource dir")
        .and_then(|path| {
            open::that(path.clone()).context(format!("fail open path {}", path.display()))
        })
        .map_err(|err| log::error!(target: "app", "[cmd]: {err}"))
        .ok();

    // core::path::AppPath::xray_preset_config_dir()
    //     .context("fail get resource dir")
    //     .and_then(|path| {
    //         open::that(path.clone()).context(format!("fail open path {}", path.display()))
    //     })
    //     .map_err(|err| log::error!(target: "app", "[cmd]: {err}"))
    //     .ok();
}

#[tauri::command]
pub fn open_log_dir() {
    core::path::AppPath::app_log_dir()
        .context("fail get log dir")
        .and_then(|path| {
            open::that(path.clone()).context(format!("fail open path {}", path.display()))
        })
        .map_err(|err| log::error!(target: "app", "[cmd]: {err}"))
        .ok();
}
