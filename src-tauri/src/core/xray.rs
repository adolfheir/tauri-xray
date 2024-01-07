use std::{fs, io::Write, str::FromStr};

use anyhow::{bail, Context, Result};
use log::logger;
use std::collections::HashSet;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use tauri::api::process::{Command, CommandChild, CommandEvent};

use super::{config::IConfig, path};

pub struct Xray {}

impl Xray {
    pub fn kill_old() -> Result<()> {
        fs::read_to_string(path::AppPath::xray_pid_path()?)
            .context("get xray pid err")
            .and_then(|pod_str| Pid::from_str(pod_str.as_str()).context(" parse pid err"))
            .map(|pid| {
                let mut system = System::new();
                system.refresh_all();
                system.process(pid).map(|proc| {
                    if proc.name().contains("clash") {
                        log::debug!(target: "app", "kill old clash process");
                        proc.kill();
                    }
                });
            })
            .ok();

        Ok(())
    }

    pub fn load() -> Result<()> {
        // `new_sidecar()` expects just the filename, NOT the whole path like in JavaScript
        let cmd = Command::new_sidecar("xray")?;
        let temp_path: std::path::PathBuf = path::AppPath::xray_temp_config_dir()?;
        let confdir = temp_path
            .to_str()
            .ok_or(anyhow::anyhow!("failed to get the config dir"))?;

        let mut remove_from_paths = Vec::new();
        remove_from_paths.push((confdir));
        fs_extra::remove_items(&remove_from_paths)?;
        fs::create_dir_all(confdir)?;

        /* 开始复制配置 */
        let mut from_paths = Vec::new();
        //复制config
        let mut temp_from_paths = Vec::new(); //TODO 研究所有权
        let preset_config_path = path::AppPath::xray_preset_config_dir()?;
        let preset_config_path = preset_config_path.as_path();
        for entry in fs::read_dir(preset_config_path)? {
            let entry = entry?;
            let path = entry.path().as_os_str().to_os_string().into_string();
            if let Ok(path) = path {
                temp_from_paths.push(path.clone())
            }
        }
        for path_string in &temp_from_paths {
            from_paths.push(path_string.as_str())
        }
        let options = fs_extra::dir::CopyOptions::new().overwrite(true);
        fs_extra::copy_items(&from_paths, confdir, &options)?;

        //复制outbound
        let outbound_path = path::AppPath::xray_outbound_dir()
            .map(|path| path.join(IConfig::active_outbound().unwrap_or_default()))?;
        let outbound_temp_path = temp_path.join("98.outbounds.json");
        let options = fs_extra::file::CopyOptions::new();
        fs_extra::file::copy(outbound_path, outbound_temp_path, &options)?;
        //复制路由
        let router_path = path::AppPath::xray_routing_dir()
            .map(|path| path.join(IConfig::active_routing().unwrap_or_default()))?;
        let router_temp_path = temp_path.join("99.routing.json");
        let options = fs_extra::file::CopyOptions::new();
        fs_extra::file::copy(router_path, router_temp_path, &options)?;

        //运行
        // see https://xtls.github.io/config/features/env.html
        // let args: Vec<&str> = vec!["-c", confdir];
        // env  XRAY_LOCATION_ASSET='/Volumes/Data/study/rust/tauri-xray/src-tauri/target/debug/resources/asset' XRAY_LOCATION_CONFDIR='/Users/chenyuhang/.config/tauri-xray/confdir'   '/Volumes/Data/study/rust/tauri-xray/src-tauri/target/debug/xray'
        let mut envs = std::collections::HashMap::new();
        envs.insert(
            "XRAY_LOCATION_ASSET".to_string(),
            path::AppPath::xray_preset_asset_dir()?
                .to_string_lossy()
                .to_string(),
        );
        envs.insert(
            "XRAY_LOCATION_CONFDIR".to_string(),
            temp_path.to_string_lossy().to_string(),
        );
        let (mut rx, cmd_child) = cmd.envs(envs).spawn()?;

        //写pid
        let pid = cmd_child.pid();
        let mut file = fs::File::create(path::AppPath::xray_pid_path()?)?;
        file.write_all(pid.to_string().as_bytes())?;

        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        // log::info!(target: "app", "[xray stdout]: {line}");
                    }
                    CommandEvent::Stderr(err) => {
                        // let stdout = clash_api::parse_log(err.clone());
                        log::warn!(target: "app", "[xray stderr]:  {err}");
                    }
                    CommandEvent::Error(err) => {
                        log::error!(target: "app", "[xray err]: {err}");
                    }
                    CommandEvent::Terminated(_) => {
                        log::warn!(target: "app", "xray core terminated");
                        break;
                    }
                    _ => {}
                }
            }
        });
        Ok(())
    }

    pub fn reload_xray() -> Result<()> {
        log::debug!("reload_xray kill");
        //关闭
        Xray::kill_old()?;

        log::debug!("reload_xray load");
        //启动
        Xray::load()?;

        log::debug!("reload_xray end");
        Ok(())
    }
}
