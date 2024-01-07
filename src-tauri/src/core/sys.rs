use anyhow::{anyhow, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use sysproxy::Sysproxy;
use tauri::{async_runtime::Mutex as TokioMutex, utils::platform::current_exe};

use crate::log_err;

use super::config::IConfig;

#[cfg(target_os = "windows")]
static DEFAULT_BYPASS: &str = "localhost;127.*;192.168.*;<local>";
#[cfg(target_os = "linux")]
static DEFAULT_BYPASS: &str = "localhost,127.0.0.1,::1";
#[cfg(target_os = "macos")]
static DEFAULT_BYPASS: &str = "127.0.0.1,localhost,<local>";

pub struct Sysopt {
    cur_sysproxy: Arc<Mutex<Option<Sysproxy>>>,
    old_sysproxy: Arc<Mutex<Option<Sysproxy>>>,
    auto_launch: Arc<Mutex<Option<AutoLaunch>>>,
    guard_state: Arc<TokioMutex<bool>>,
}

impl Sysopt {
    pub fn global() -> &'static Sysopt {
        static SYSOPT: OnceCell<Sysopt> = OnceCell::new();

        SYSOPT.get_or_init(|| Sysopt {
            cur_sysproxy: Arc::new(Mutex::new(None)),
            old_sysproxy: Arc::new(Mutex::new(None)),
            auto_launch: Arc::new(Mutex::new(None)),
            guard_state: Arc::new(TokioMutex::new(false)),
        })
    }

    pub fn init_sysproxy(&self) -> Result<()> {
        let port_config =
            IConfig::port_config().ok_or(anyhow::anyhow!("failed to get port config"))?;
        let port_enable = IConfig::sys_port_enable()
            .ok_or(anyhow::anyhow!("failed to get port enable config"))?;
        let http_port = port_config
            .http_port
            .ok_or(anyhow::anyhow!("failed to get http port"))?;

        let current = Sysproxy {
            enable: port_enable,
            host: String::from("127.0.0.1"),
            port: http_port,
            bypass: DEFAULT_BYPASS.into(),
        };

        if port_enable {
            let old = Sysproxy::get_system_proxy().map_or(None, |p| Some(p));
            current.set_system_proxy()?;

            *self
                .old_sysproxy
                .lock()
                .map_err(|_| anyhow!("Mutex lock error"))? = old;
            *self
                .cur_sysproxy
                .lock()
                .map_err(|_| anyhow!("Mutex lock error"))? = Some(current);
        }

        // self.guard_proxy();
        Ok(())
    }

    pub fn update_sysproxy(&self) -> Result<()> {
        let mut cur_sysproxy = self
            .cur_sysproxy
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))?;
        let old_sysproxy = self
            .old_sysproxy
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))?;

        if cur_sysproxy.is_none() || old_sysproxy.is_none() {
            drop(cur_sysproxy);
            drop(old_sysproxy);
            return self.init_sysproxy();
        }

        let port_config =
            IConfig::port_config().ok_or(anyhow::anyhow!("failed to get port config"))?;
        let port_enable = IConfig::sys_port_enable()
            .ok_or(anyhow::anyhow!("failed to get port enable config"))?;
        let http_port = port_config
            .http_port
            .ok_or(anyhow::anyhow!("failed to get http port"))?;

        let mut sysproxy = cur_sysproxy.take().unwrap();
        sysproxy.enable = port_enable;
        sysproxy.port = http_port;
        sysproxy.bypass = DEFAULT_BYPASS.into();

        sysproxy.set_system_proxy()?;
        *cur_sysproxy = Some(sysproxy);

        Ok(())
    }

    pub fn reset_sysproxy(&self) -> Result<()> {
        let mut cur_sysproxy = self
            .cur_sysproxy
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))?;
        let mut old_sysproxy = self
            .old_sysproxy
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))?;

        let cur_sysproxy = cur_sysproxy.take();

        if let Some(mut old) = old_sysproxy.take() {
            let port_same = cur_sysproxy.map_or(true, |cur| old.port == cur.port);

            if old.enable && port_same {
                old.enable = false;
                log::info!(target: "app", "reset proxy by disabling the original proxy");
            } else {
                log::info!(target: "app", "reset proxy to the original proxy");
            }

            old.set_system_proxy()?;
        } else if let Some(mut cur @ Sysproxy { enable: true, .. }) = cur_sysproxy {
            log::info!(target: "app", "reset proxy by disabling the current proxy");
            cur.enable = false;
            cur.set_system_proxy()?;
        } else {
            log::info!(target: "app", "reset proxy with no action");
        }

        Ok(())
    }

    pub fn init_launch(&self) -> Result<()> {
        let enable = IConfig::auto_launch_enable()
            .ok_or(anyhow::anyhow!("failed to get port enable config"))?;

        let app_exe = current_exe()?;
        let app_exe = dunce::canonicalize(app_exe)?;
        let app_name = app_exe
            .file_stem()
            .and_then(|f| f.to_str())
            .ok_or(anyhow!("failed to get file stem"))?;

        let app_path = app_exe
            .as_os_str()
            .to_str()
            .ok_or(anyhow!("failed to get app_path"))?
            .to_string();

        #[cfg(target_os = "windows")]
        let app_path = format!("\"{app_path}\"");

        #[cfg(target_os = "macos")]
        let app_path = (|| -> Option<String> {
            let path = std::path::PathBuf::from(&app_path);
            let path = path.parent()?.parent()?.parent()?;
            let extension = path.extension()?.to_str()?;
            match extension == "app" {
                true => Some(path.as_os_str().to_str()?.to_string()),
                false => None,
            }
        })()
        .unwrap_or(app_path);

        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(&app_path)
            .build()?;

        #[cfg(feature = "verge-dev")]
        if !enable {
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            if enable && !auto.is_enabled().unwrap_or(false) {
                let _ = auto.disable();
                auto.enable()?;
            } else if !enable {
                let _ = auto.disable();
            }
        }

        #[cfg(not(target_os = "macos"))]
        if enable {
            auto.enable()?;
        }

        *self
            .auto_launch
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))? = Some(auto);

        Ok(())
    }

    pub fn update_launch(&self) -> Result<()> {
        let auto_launch = self
            .auto_launch
            .lock()
            .map_err(|_| anyhow!("Mutex lock error"))?;

        if auto_launch.is_none() {
            drop(auto_launch);
            return self.init_launch();
        }
        let enable = IConfig::auto_launch_enable()
            .ok_or(anyhow::anyhow!("failed to get port enable config"))?;
        let auto_launch = auto_launch.as_ref().unwrap();

        match enable {
            true => auto_launch.enable()?,
            false => log_err!(auto_launch.disable()),
        };

        Ok(())
    }

    pub fn guard_proxy(&self) {
        use tokio::time::{sleep, Duration};

        let guard_state = self.guard_state.clone();

        tauri::async_runtime::spawn(async move {
            let mut state = guard_state.lock().await;
            if *state {
                return;
            }
            *state = true;
            drop(state);

            let mut wait_secs = 10u64;

            loop {
                sleep(Duration::from_secs(wait_secs)).await;

                let port_enable = IConfig::sys_port_enable().unwrap_or(false);
                let http_port = IConfig::port_config()
                    .and_then(|v| v.http_port)
                    .unwrap_or(10809);

                if !port_enable {
                    break;
                }

                wait_secs = 10;

                log::debug!(target: "app", "try to guard the system proxy");

                let sysproxy = Sysproxy {
                    enable: true,
                    host: "127.0.0.1".into(),
                    port: http_port,
                    bypass: DEFAULT_BYPASS.into(),
                };

                log_err!(sysproxy.set_system_proxy());
            }

            let mut state = guard_state.lock().await;
            *state = false;
            drop(state);
        });
    }
}
