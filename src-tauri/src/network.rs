use crate::types::{JlcError, NetworkSettings};
use std::sync::{Mutex, OnceLock};

static NETWORK_SETTINGS: OnceLock<Mutex<NetworkSettings>> = OnceLock::new();

fn network_settings_store() -> &'static Mutex<NetworkSettings> {
    NETWORK_SETTINGS.get_or_init(|| Mutex::new(NetworkSettings::default()))
}

pub fn get_network_settings() -> NetworkSettings {
    network_settings_store()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

pub fn set_network_settings(settings: NetworkSettings) -> Result<(), JlcError> {
    let proxy_addr = settings.proxy_address.trim();

    if settings.easyeda_use_proxy && !proxy_addr.is_empty() {
        reqwest::Proxy::all(proxy_addr)
            .map_err(|e| JlcError::ApiError(format!("代理地址无效: {}", e)))?;
    }

    if settings.lcsc_use_proxy && !proxy_addr.is_empty() {
        reqwest::Proxy::all(proxy_addr)
            .map_err(|e| JlcError::ApiError(format!("代理地址无效: {}", e)))?;
    }

    match network_settings_store().lock() {
        Ok(mut state) => {
            *state = settings;
            Ok(())
        }
        Err(_) => Err(JlcError::ApiError("无法写入网络设置".to_string())),
    }
}
