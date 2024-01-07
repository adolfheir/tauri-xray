use crate::{cmds, core::config::IConfig};
use anyhow::Result;
use log::log;
use tauri::{
    api::{self, path::config_dir},
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, SystemTraySubmenu,
};

use super::{sys::Sysopt, xray};

pub struct Tray {}

impl Tray {
    // 托盘菜单
    pub fn menu() -> SystemTrayMenu {
        let zh = true;

        let version = "0.0.0".to_string();

        macro_rules! t {
            ($en: expr, $zh: expr) => {
                if zh {
                    $zh
                } else {
                    $en
                }
            };
        }

        //路由
        let mut router_menu: SystemTrayMenu = SystemTrayMenu::new();
        let select_router: Option<String> = IConfig::active_routing().map(|v| v.clone());
        if let Some(router_list) = IConfig::get_routing_list() {
            for pathbuf in router_list {
                let file_name = pathbuf.file_name().and_then(|file_name| file_name.to_str());
                match file_name {
                    Some(name) => {
                        let mut is_selected = false;

                        if let Some(ref select_router_name) = select_router {
                            is_selected = select_router_name.as_str() == name;
                        }

                        let item_id = format!("{}{}", "router_", name);
                        let mut item = CustomMenuItem::new(item_id, name);
                        if (is_selected) {
                            item = item.selected()
                        }

                        router_menu = router_menu.add_item(item)
                    }
                    None => {}
                }
            }
        }

        //outbound
        let mut outbound_menu: SystemTrayMenu = SystemTrayMenu::new();
        let select_outbound: Option<String> = IConfig::active_outbound().map(|v| v.clone());
        if let Some(outbound_list) = IConfig::get_outbound_list() {
            for pathbuf in outbound_list {
                let file_name = pathbuf.file_name().and_then(|file_name| file_name.to_str());
                match file_name {
                    Some(name) => {
                        let mut is_selected = false;

                        if let Some(ref select_outbound_name) = select_outbound {
                            is_selected = select_outbound_name.as_str() == name;
                        }

                        let item_id = format!("{}{}", "outbound_", name);
                        let mut item = CustomMenuItem::new(item_id, name);
                        if (is_selected) {
                            item = item.selected()
                        }

                        outbound_menu = outbound_menu.add_item(item)
                    }
                    None => {}
                }
            }
        }

        //sys proxy
        let mut sys_port_menu = CustomMenuItem::new("system_proxy", "系统代理");
        let is_sys_port_select = IConfig::sys_port_enable().unwrap_or(true);
        if is_sys_port_select {
            sys_port_menu = sys_port_menu.selected()
        }

        let tray_menu: SystemTrayMenu = SystemTrayMenu::new()
            .add_item(sys_port_menu)
            .add_item(CustomMenuItem::new(
                "restart_xray",
                t!("Restart Xray", "重启 Xray"),
            ))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_submenu(SystemTraySubmenu::new("路由切换", router_menu))
            .add_submenu(SystemTraySubmenu::new("outbound切换", outbound_menu))
            .add_submenu(SystemTraySubmenu::new(
                t!("Open Dir", "打开目录"),
                SystemTrayMenu::new()
                    .add_item(CustomMenuItem::new(
                        "open_app_dir",
                        t!("App Dir", "应用目录"),
                    ))
                    .add_item(CustomMenuItem::new(
                        "open_core_dir",
                        t!("Core Dir", "内核目录"),
                    ))
                    .add_item(CustomMenuItem::new(
                        "open_logs_dir",
                        t!("Logs Dir", "日志目录"),
                    )),
            ))
            .add_submenu(SystemTraySubmenu::new(
                t!("More", "更多"),
                SystemTrayMenu::new()
                    .add_item(CustomMenuItem::new("refresh", t!("refresh", "刷新")))
                    .add_item(CustomMenuItem::new(
                        "copy_env",
                        t!("Copy Env", "复制环境变量"),
                    ))
                    .add_item(
                        CustomMenuItem::new("app_version", format!("Version {version}")).disabled(),
                    ),
            ))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(
                CustomMenuItem::new("quit", t!("Quit", "退出")).accelerator("CmdOrControl+Q"),
            );

        return tray_menu;
    }

    pub fn update_tray(app_handle: &AppHandle) -> Result<()> {
        let menu = Tray::menu();
        app_handle.tray_handle().set_menu(menu)?;
        Ok(())
    }

    // 菜单事件
    pub fn handler(app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "restart_xray" => cmds::restart_xray(),
                "open_app_dir" => cmds::open_app_home_dir(),
                "open_core_dir" => cmds::open_core_dir(),
                "open_logs_dir" => cmds::open_log_dir(),
                "system_proxy" => {
                    let enable: bool = IConfig::sys_port_enable().unwrap_or(true);
                    IConfig::set_sys_port_enable(!enable);
                    Sysopt::global().update_sysproxy();
                    Tray::update_tray(app);
                }
                "quit" => {
                    Sysopt::global().reset_sysproxy();
                    api::process::kill_children();
                    app.exit(0);
                    std::process::exit(0);
                }
                "refresh" => {
                    Tray::update_tray(&app.app_handle());
                }
                s if s.starts_with("router_") => {
                    if let Some(rest_of_string) = s.strip_prefix("router_") {
                        log::error!(target: "str1", "{rest_of_string}");
                        IConfig::set_active_routing(rest_of_string.to_string());
                        log::error!(target: "str2", "{rest_of_string}");
                        Tray::update_tray(app);
                        log::error!(target: "str3", "{rest_of_string}");
                        xray::Xray::reload_xray();
                    }
                }
                s if s.starts_with("outbound_") => {
                    // 处理以 "prefix1" 开头的逻辑
                    if let Some(rest_of_string) = s.strip_prefix("outbound_") {
                        IConfig::set_active_outbound(rest_of_string.to_string());
                        Tray::update_tray(app);
                        xray::Xray::reload_xray().unwrap()
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}
