//! Application settings Tauri commands.

use std::time::Duration;

use crate::settings::{
    build_reqwest_proxy, clear_proxy_settings as clear_stored_proxy_settings,
    get_proxy_settings_info, load_settings, parse_proxy_settings,
    set_proxy_settings as save_proxy_settings, ProxySettingsInfo,
};

const PROXY_TEST_URL: &str = "https://auth.openai.com/.well-known/openid-configuration";

#[tauri::command]
pub async fn get_proxy_settings() -> Result<ProxySettingsInfo, String> {
    get_proxy_settings_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_proxy_settings(
    proxy: Option<String>,
    enabled: bool,
) -> Result<ProxySettingsInfo, String> {
    save_proxy_settings(proxy, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_proxy_settings() -> Result<ProxySettingsInfo, String> {
    clear_stored_proxy_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_proxy_settings(proxy: Option<String>) -> Result<(), String> {
    let proxy_settings = match proxy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => parse_proxy_settings(value).map_err(|e| e.to_string())?,
        None => load_settings()
            .map_err(|e| e.to_string())?
            .proxy
            .ok_or_else(|| "Proxy is not configured".to_string())?,
    };

    let client = reqwest::Client::builder()
        .proxy(build_reqwest_proxy(&proxy_settings).map_err(|e| e.to_string())?)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build proxy test client: {e}"))?;

    let response = client
        .get(PROXY_TEST_URL)
        .send()
        .await
        .map_err(|e| format!("Proxy test request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Proxy test failed with status {}",
            response.status()
        ));
    }

    Ok(())
}
