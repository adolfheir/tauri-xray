use crate::core::path;
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Mutex;

use super::path::AppPath;

/* 结构体 */
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Inbound {
    port: u16,
    protocol: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct InboundsConfigData {
    inbounds: Vec<Inbound>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserConfigValue {
    active_routing: String,
    active_outbound: String,
    sys_port_enable: bool,
    auto_launch_enable: bool, // 新增的字段
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortConfig {
    pub http_port: Option<u16>,
    pub socks_port: Option<u16>,
}

/* 全局变量 */
lazy_static! {
    static ref PORT_CONFIG: Mutex<Option<PortConfig>> = Mutex::new(None);
    static ref ACTIVE_ROUTING: Mutex<Option<String>> = Mutex::new(None);
    static ref ACTIVE_OUTBOUND: Mutex<Option<String>> = Mutex::new(None);
    static ref SYS_PORT_ENABLE: Mutex<Option<bool>> = Mutex::new(Some(false));
    static ref AUTO_LAUNCH_ENABLE: Mutex<Option<bool>> = Mutex::new(Some(true)); // 新增的全局变量
}

pub struct IConfig {}

impl IConfig {
    pub fn active_routing() -> Option<String> {
        ACTIVE_ROUTING.lock().ok().and_then(|v| v.clone())
    }

    pub fn active_outbound() -> Option<String> {
        ACTIVE_OUTBOUND.lock().ok().and_then(|v| v.clone())
    }

    pub fn sys_port_enable() -> Option<bool> {
        SYS_PORT_ENABLE.lock().ok().and_then(|v| *v)
    }

    pub fn auto_launch_enable() -> Option<bool> {
        AUTO_LAUNCH_ENABLE.lock().ok().and_then(|v| *v)
    }

    pub fn port_config() -> Option<PortConfig> {
        PORT_CONFIG.lock().ok().and_then(|v| v.clone())
    }

    pub fn init_config() -> Result<()> {
        let user_config_json = IConfig::get_init_user_config();

        let active_routing = user_config_json.active_routing;
        ACTIVE_ROUTING
            .lock()
            .map(|mut v| *v = Some(active_routing))
            .ok();

        let active_outbound = user_config_json.active_outbound;
        ACTIVE_OUTBOUND
            .lock()
            .map(|mut v| *v = Some(active_outbound))
            .ok();

        let sys_port_enable = user_config_json.sys_port_enable;
        SYS_PORT_ENABLE
            .lock()
            .map(|mut v| *v = Some(sys_port_enable))
            .ok();

        let auto_launch_enable = user_config_json.auto_launch_enable;
        AUTO_LAUNCH_ENABLE
            .lock()
            .map(|mut v| *v = Some(auto_launch_enable))
            .ok();

        let port_config = IConfig::get_init_port_config();
        PORT_CONFIG.lock().map(|mut v| *v = Some(port_config)).ok();

        Ok(())
    }

    pub fn set_active_routing(new_data: String) -> Result<()> {
        ACTIVE_ROUTING.lock().map(|mut v| *v = Some(new_data)).ok();
        IConfig::write_config()?;
        Ok(())
    }

    pub fn set_active_outbound(new_data: String) -> Result<()> {
        ACTIVE_OUTBOUND.lock().map(|mut v| *v = Some(new_data)).ok();
        IConfig::write_config()?;
        Ok(())
    }
    pub fn set_sys_port_enable(new_data: bool) -> Result<()> {
        SYS_PORT_ENABLE.lock().map(|mut v| *v = Some(new_data)).ok();
        IConfig::write_config()?;
        Ok(())
    }

    pub fn write_config() -> Result<()> {
        let new_config = UserConfigValue {
            active_routing: IConfig::active_routing().unwrap_or_default(),
            active_outbound: IConfig::active_outbound().unwrap_or_default(),
            sys_port_enable: IConfig::sys_port_enable().unwrap_or_default(),
            auto_launch_enable: IConfig::auto_launch_enable().unwrap_or_default(),
        };
        let json_str = serde_json::to_string(&new_config)?;

        let config_path = AppPath::config_json()?;
        fs::write(config_path, json_str.as_bytes())?;

        Ok(())
    }

    pub fn get_init_user_config() -> UserConfigValue {
        AppPath::config_json()
            .ok()
            .and_then(|file_path| File::open(file_path).ok())
            .map(BufReader::new)
            .and_then(|reader| {
                let result: Result<UserConfigValue, _> = serde_json::from_reader(reader);
                result.ok()
            })
            .unwrap_or(UserConfigValue {
                active_routing: String::default(),
                active_outbound: String::default(),
                sys_port_enable: true,
                auto_launch_enable: true,
            })
    }

    pub fn get_init_port_config() -> PortConfig {
        let parsed_data = path::AppPath::xray_preset_config_dir()
            .map(|path| path.join("05_inbounds.json"))
            .ok()
            .and_then(|file_path| fs::read_to_string(file_path).ok())
            .and_then(|json_str| {
                let result: Result<InboundsConfigData, _> = serde_json::from_str(json_str.as_str());
                result.ok()
            });

        let http_port: Option<u16> = parsed_data.clone().and_then(|parsed_data| {
            parsed_data
                .inbounds
                .iter()
                .find(|inbound| inbound.protocol == "http")
                .map(|inbound| inbound.port)
        });

        let socket_port = parsed_data.clone().and_then(|parsed_data| {
            parsed_data
                .inbounds
                .iter()
                .find(|inbound| inbound.protocol == "socks")
                .map(|inbound| inbound.port)
        });

        PortConfig {
            http_port,
            socks_port: socket_port,
        }
    }

    pub fn get_routing_list() -> Option<Vec<PathBuf>> {
        let path_list: Option<Vec<PathBuf>> = path::AppPath::xray_routing_dir()
            .ok()
            .and_then(|path| fs::read_dir(path).ok())
            .map(|entries| {
                let file_paths: Vec<PathBuf> = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.path().is_file() && entry.path().extension() == Some("json".as_ref())
                    })
                    .map(|entry| entry.path())
                    .collect();
                file_paths
            });
        path_list
    }

    pub fn get_outbound_list() -> Option<Vec<PathBuf>> {
        let path_list: Option<Vec<PathBuf>> = path::AppPath::xray_outbound_dir()
            .ok()
            .and_then(|path| fs::read_dir(path).ok())
            .map(|entries| {
                let file_paths: Vec<PathBuf> = entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .collect();
                file_paths
            });
        path_list
    }
}
