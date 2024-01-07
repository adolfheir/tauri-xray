use anyhow::{Context, Ok, Result};
use fs_extra;
use once_cell::sync::OnceCell;
use std::fs;
use std::path::PathBuf;
use tauri::{
    api::path::{home_dir, resource_dir},
    App,
};

static APP_DIR: &str = "tauri-xray";
static XRAY_PID: &str = "xray.pid";
static CONFIG_JSON: &str = "conig.json";

//维护全局 resource dir
pub static RESOLVE: OnceCell<tauri::PathResolver> = OnceCell::new();

pub struct AppPath {}

impl AppPath {
    pub fn init_path(resolver: tauri::PathResolver) -> Result<()> {
        //缓存resolve
        RESOLVE.set(resolver).unwrap();
        //在这边初始化home目录
        let app_dir = AppPath::app_home_dir()?;
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }

        let app_log_dir = AppPath::app_log_dir()?;
        if !app_log_dir.exists() {
            fs::create_dir_all(&app_log_dir)?;
        }

        //复制路由
        let xray_routing_dir = AppPath::xray_routing_dir()?;
        if !xray_routing_dir.exists() {
            fs::create_dir_all(&xray_routing_dir)?;
            let preset_path = AppPath::app_core_dir().map(|path| path.join("routing"))?;
            let preset_path_str = preset_path
                .to_str()
                .ok_or(anyhow::anyhow!("failed to get the app home dir"))?;

            let options = fs_extra::dir::CopyOptions::new().skip_exist(true);
            log::info!("from:{},to:{}", xray_routing_dir.display(),preset_path.display());
            fs_extra::dir::copy(preset_path_str, app_dir.clone(), &options)
                .with_context(|| format!("Failed to read xray_routing_dir, {}", preset_path_str))?;
        }
        //复制outbound
        let xray_outbound_dir = AppPath::xray_outbound_dir()?;
        
        if !xray_outbound_dir.exists() {
            // fs::create_dir_all(&xray_outbound_dir);
            let preset_path = AppPath::app_core_dir().map(|path| path.join("outbound"))?;
            let preset_path_str = preset_path
                .to_str()
                .ok_or(anyhow::anyhow!("failed to get the app home dir"))?;

            let options = fs_extra::dir::CopyOptions::new().skip_exist(true);
            log::info!("from:{},to:{}", xray_outbound_dir.display(),preset_path.display());
            fs_extra::dir::copy(preset_path_str, app_dir.clone(), &options)
                .with_context(|| format!("Failed to read xray_outbound_dir {}", preset_path_str))?;
        }

        Ok(())
    }

    /* 目录 */
    pub fn app_core_dir() -> Result<PathBuf> {
        let res_dir: PathBuf = RESOLVE
            .get()
            .and_then(|resolve| resolve.resource_dir())
            .ok_or(anyhow::anyhow!("failed to get the app home dir"))?
            .join("resources");

        Ok(res_dir)
    }

    pub fn app_home_dir() -> Result<PathBuf> {
        Ok(home_dir()
            .ok_or(anyhow::anyhow!("failed to get the app home dir"))?
            .join(".config")
            .join(APP_DIR))
    }

    pub fn app_log_dir() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join("logs"))
    }

    /* 用户配置文件路径 */
    pub fn xray_preset_config_dir() -> Result<PathBuf> {
        Ok(AppPath::app_core_dir()?.join("confdir"))
    }
    /* 使用的配置文件 */
    pub fn xray_temp_config_dir() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join("confdir"))
    }
    /* 用户资源文件路径 */
    pub fn xray_preset_asset_dir() -> Result<PathBuf> {
        Ok(AppPath::app_core_dir()?.join("asset"))
    }

    /* 路由 */
    pub fn xray_routing_dir() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join("routing"))
    }
    /* 代理地址 */
    pub fn xray_outbound_dir() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join("outbound"))
    }

    /* 文件 */
    pub fn xray_pid_path() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join(XRAY_PID))
    }
    /* 使用的路由 服务器 */
    pub fn config_json() -> Result<PathBuf> {
        Ok(AppPath::app_home_dir()?.join(CONFIG_JSON))
    }
}
