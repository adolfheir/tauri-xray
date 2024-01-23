use anyhow::{Ok, Result};

use rustem_proxy::SystemProxy;




use super::config::IConfig;

#[cfg(target_os = "windows")]
static DEFAULT_BYPASS: &str = "localhost;127.*;192.168.*;<local>";
#[cfg(target_os = "macos")]
static DEFAULT_BYPASS: &str = "127.0.0.1,localhost,<local>";

pub struct Sysopt {}

impl Sysopt {
    // pub fn able_proxy() -> Result<()> {
    pub fn able_proxy() -> Result<()> {
        let port_config =
            IConfig::port_config().ok_or(anyhow::anyhow!("failed to get port config"))?;
        let http_port = port_config
            .http_port
            .ok_or(anyhow::anyhow!("failed to get http port"))?;
        let socket_port = port_config
            .socks_port
            .ok_or(anyhow::anyhow!("failed to get socket port"))?;

        // http port
        SystemProxy::set(SystemProxy {
            is_enabled: true,
            host: "127.0.0.1".to_string(),
            port: http_port,
            bypass: DEFAULT_BYPASS.to_string(),
            protocol: rustem_proxy::Protocol::HTTP,
        });
        SystemProxy::set(SystemProxy {
            is_enabled: true,
            host: "127.0.0.1".to_string(),
            port: http_port,
            bypass: DEFAULT_BYPASS.to_string(),
            protocol: rustem_proxy::Protocol::HTTPS,
        });
        SystemProxy::set(SystemProxy {
            is_enabled: true,
            host: "127.0.0.1".to_string(),
            port: socket_port,
            bypass: DEFAULT_BYPASS.to_string(),
            protocol: rustem_proxy::Protocol::SOCKS,
        });

        Ok(())
    }

    pub fn disable_proxy() -> Result<()> {
        SystemProxy::unset();
        Ok(())
    }

    pub fn sync_proxy() -> Result<()> {
        let port_enable = IConfig::sys_port_enable()
            .ok_or(anyhow::anyhow!("failed to get port enable config"))?;

        if port_enable {
            Sysopt::able_proxy()?;
        } else {
            Sysopt::disable_proxy()?;
        }
        Ok(())
    }
}
