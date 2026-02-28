#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use jlc2kicad_tauri_lib::{
    create_component, search_easyeda as do_easyeda, search_lcsc as do_lcsc,
    load_local_folder as do_load, SearchResult, NetworkSettings,
    get_network_settings as get_net_settings, set_network_settings as set_net_settings,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
#[cfg(debug_assertions)]
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateComponentOptions {
    pub component_id: String,
    pub output_dir: String,
    pub footprint_lib: String,
    pub symbol_lib: String,
    pub symbol_path: String,
    pub model_dir: String,
    pub models: Vec<String>,
    pub create_footprint: bool,
    pub create_symbol: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalOptions {
    pub path: String,
    pub output_dir: String,
    pub footprint_lib: String,
    pub symbol_lib: String,
    pub symbol_path: String,
    pub model_dir: String,
    pub models: Vec<String>,
    pub create_footprint: bool,
    pub create_symbol: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

#[tauri::command]
async fn create_component_cmd(
    options: CreateComponentOptions,
    window: tauri::Window,
) -> Result<CommandResult, String> {
    let component_id = options.component_id.clone();
    
    window.emit("progress", "正在创建元件...").ok();

    match create_component(
        &options.component_id,
        &options.output_dir,
        &options.footprint_lib,
        &options.symbol_lib,
        &options.symbol_path,
        &options.model_dir,
        options.models,
        options.create_footprint,
        options.create_symbol,
    )
    .await
    {
        Ok(message) => {
            window.emit("progress", &message).ok();
            Ok(CommandResult {
                success: true,
                message,
                error: None,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            window.emit("error", &error_msg).ok();
            Ok(CommandResult {
                success: false,
                message: format!("创建元件 {} 失败", component_id),
                error: Some(error_msg),
            })
        }
    }
}

#[tauri::command]
fn get_default_output_dir() -> String {
    dirs::document_dir()
        .map(|p| p.join("JLC2KiCad_lib").to_string_lossy().to_string())
        .unwrap_or_else(|| "JLC2KiCad_lib".to_string())
}

#[tauri::command]
async fn search_easyeda_cmd(query: String) -> Result<Vec<SearchResult>, String> {
    do_easyeda(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_lcsc(query: String) -> Result<Vec<SearchResult>, String> {
    do_lcsc(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn load_local_folder(path: String) -> Result<Vec<SearchResult>, String> {
    do_load(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn convert_local(
    options: LocalOptions,
    window: tauri::Window,
) -> Result<CommandResult, String> {
    window.emit("progress", "正在转换本地文件...").ok();
    
    match jlc2kicad_tauri_lib::convert_local_folder(
        &options.path,
        &options.output_dir,
        &options.footprint_lib,
        &options.symbol_lib,
        &options.symbol_path,
        &options.model_dir,
        options.models,
        options.create_footprint,
        options.create_symbol,
    )
    .await
    {
        Ok(message) => {
            window.emit("progress", &message).ok();
            Ok(CommandResult {
                success: true,
                message,
                error: None,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            Ok(CommandResult {
                success: false,
                message: "转换失败".to_string(),
                error: Some(error_msg),
            })
        }
    }
}

#[tauri::command]
fn get_network_settings_cmd() -> NetworkSettings {
    get_net_settings()
}

#[tauri::command]
fn set_network_settings_cmd(settings: NetworkSettings) -> Result<CommandResult, String> {
    match set_net_settings(settings) {
        Ok(_) => Ok(CommandResult {
            success: true,
            message: "网络设置已保存".to_string(),
            error: None,
        }),
        Err(e) => Ok(CommandResult {
            success: false,
            message: "保存网络设置失败".to_string(),
            error: Some(e.to_string()),
        }),
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting JLC2KiCad application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = _app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_component_cmd,
            get_default_output_dir,
            search_easyeda_cmd,
            search_lcsc,
            load_local_folder,
            convert_local,
            get_network_settings_cmd,
            set_network_settings_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
