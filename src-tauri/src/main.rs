// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

mod cmds;
mod core;
mod macros;

use tauri::{App, AppHandle, Manager, SystemTray};
use tauri::{Menu, MenuItem, Submenu, WindowBuilder};
use tauri_plugin_autostart::MacosLauncher;

use crate::core::config::IConfig;
use crate::core::sys::Sysopt;
use crate::core::tray::Tray;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

fn main() {
    env_logger::init();

    log::debug!("Initialized logger");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);

            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--flag1", "--flag2"]), /* arbitrary number of args to pass to your app */
        ))
        .system_tray(SystemTray::new())
        .on_system_tray_event(core::tray::Tray::handler)
        .invoke_handler(tauri::generate_handler![cmds::greet,])
        .setup(|app: &mut App| Ok(setup_app(app)))
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, e| match e {
        tauri::RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        tauri::RunEvent::Exit => {
            // resolve::resolve_reset();
            // api::process::kill_children();
            app_handle.exit(0);
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::WindowEvent { label, event, .. } => {
            if label == "main" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        app_handle.get_window("main").map(|win| {
                            let _ = win.hide();
                        });
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}

//启动app
fn setup_app(app: &mut App) {
    // 初始化文件目录
    log_err!(core::path::AppPath::init_path(app.path_resolver()));

    // 初始化配置
    log_err!(IConfig::init_config());

    // 初始化系统相关
    log_err!(Sysopt::global().init_launch());
    log_err!(Sysopt::global().init_sysproxy());

    // 初始化xray进程
    log_err!(core::xray::Xray::reload_xray());

    // 初始化tray
    // 设置没有菜单栏，只有系统托盘图标
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    log_err!(Tray::update_tray(&app.app_handle()));
}
