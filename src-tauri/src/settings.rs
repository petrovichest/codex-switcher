//! Application settings storage and proxy helpers.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::get_config_dir;

const PROXY_ENV_KEYS: [&str; 6] = [
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
];

static ORIGINAL_PROXY_ENV: OnceLock<HashMap<&'static str, Option<OsString>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub version: u32,
    #[serde(default)]
    pub proxy: Option<ProxySettings>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            proxy: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxySettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxySettingsInfo {
    pub enabled: bool,
    pub configured: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub has_password: bool,
}

impl ProxySettingsInfo {
    fn empty() -> Self {
        Self {
            enabled: false,
            configured: false,
            host: None,
            port: None,
            username: None,
            has_password: false,
        }
    }
}

impl From<Option<ProxySettings>> for ProxySettingsInfo {
    fn from(proxy: Option<ProxySettings>) -> Self {
        match proxy {
            Some(proxy) => Self {
                enabled: proxy.enabled,
                configured: true,
                host: Some(proxy.host),
                port: Some(proxy.port),
                username: proxy.username,
                has_password: proxy.password.is_some(),
            },
            None => Self::empty(),
        }
    }
}

pub fn get_settings_file() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("settings.json"))
}

pub fn load_settings() -> Result<AppSettings> {
    let path = get_settings_file()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read settings file: {}", path.display()))?;
    let settings = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse settings file: {}", path.display()))?;
    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let path = get_settings_file()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write settings file: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

pub fn get_proxy_settings_info() -> Result<ProxySettingsInfo> {
    Ok(ProxySettingsInfo::from(load_settings()?.proxy))
}

pub fn set_proxy_settings(proxy_input: Option<String>, enabled: bool) -> Result<ProxySettingsInfo> {
    let mut settings = load_settings()?;

    if let Some(proxy_input) = proxy_input {
        let trimmed = proxy_input.trim();
        if !trimmed.is_empty() {
            let mut proxy = parse_proxy_settings(trimmed)?;
            proxy.enabled = enabled;
            settings.proxy = Some(proxy);
        } else if let Some(proxy) = settings.proxy.as_mut() {
            proxy.enabled = enabled;
        } else if enabled {
            anyhow::bail!("Proxy string is required to enable proxy");
        }
    } else if let Some(proxy) = settings.proxy.as_mut() {
        proxy.enabled = enabled;
    } else if enabled {
        anyhow::bail!("Proxy string is required to enable proxy");
    }

    save_settings(&settings)?;
    apply_proxy_environment(settings.proxy.as_ref())?;
    Ok(ProxySettingsInfo::from(settings.proxy))
}

pub fn clear_proxy_settings() -> Result<ProxySettingsInfo> {
    let mut settings = load_settings()?;
    settings.proxy = None;
    save_settings(&settings)?;
    apply_proxy_environment(None)?;
    Ok(ProxySettingsInfo::empty())
}

pub fn replace_proxy_settings(proxy: Option<ProxySettings>) -> Result<ProxySettingsInfo> {
    if let Some(proxy) = &proxy {
        validate_proxy_settings(proxy)?;
    }

    let mut settings = load_settings()?;
    settings.proxy = proxy;
    save_settings(&settings)?;
    apply_proxy_environment(settings.proxy.as_ref())?;
    Ok(ProxySettingsInfo::from(settings.proxy))
}

pub fn apply_stored_proxy_environment() -> Result<()> {
    apply_proxy_environment(load_settings()?.proxy.as_ref())
}

pub fn build_http_client() -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    if let Some(proxy) = load_settings()?.proxy.filter(|proxy| proxy.enabled) {
        builder = builder.proxy(build_reqwest_proxy(&proxy)?);
    }

    builder.build().context("Failed to build HTTP client")
}

pub fn build_reqwest_proxy(proxy: &ProxySettings) -> Result<Proxy> {
    let proxy_url = build_proxy_url(proxy, false)?;
    let reqwest_proxy = Proxy::all(&proxy_url)
        .with_context(|| format!("Failed to configure HTTP proxy: {proxy_url}"))?;

    match (&proxy.username, &proxy.password) {
        (Some(username), Some(password)) => Ok(reqwest_proxy.basic_auth(username, password)),
        (None, None) => Ok(reqwest_proxy),
        _ => anyhow::bail!("Proxy username and password must be provided together"),
    }
}

fn apply_proxy_environment(proxy: Option<&ProxySettings>) -> Result<()> {
    let original_env = ORIGINAL_PROXY_ENV.get_or_init(|| {
        PROXY_ENV_KEYS
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect()
    });

    match proxy.filter(|proxy| proxy.enabled) {
        Some(proxy) => {
            let proxy_url = build_proxy_url(proxy, true)?;
            for key in PROXY_ENV_KEYS {
                std::env::set_var(key, &proxy_url);
            }
        }
        None => {
            for key in PROXY_ENV_KEYS {
                match original_env.get(key).and_then(|value| value.as_ref()) {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    Ok(())
}

fn build_proxy_url(proxy: &ProxySettings, include_auth: bool) -> Result<String> {
    let host = if proxy.host.contains(':') && !proxy.host.starts_with('[') {
        format!("[{}]", proxy.host)
    } else {
        proxy.host.clone()
    };
    let mut url = Url::parse(&format!("http://{host}:{}", proxy.port))
        .context("Failed to build proxy URL")?;

    if include_auth {
        if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
            url.set_username(username)
                .map_err(|_| anyhow::anyhow!("Invalid proxy username"))?;
            url.set_password(Some(password))
                .map_err(|_| anyhow::anyhow!("Invalid proxy password"))?;
        }
    }

    Ok(url.to_string())
}

pub fn parse_proxy_settings(input: &str) -> Result<ProxySettings> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Proxy string is required");
    }

    if trimmed.contains("://") {
        return parse_proxy_url(trimmed);
    }

    if trimmed.contains('@') {
        if let Ok(proxy) = parse_proxy_parts(trimmed) {
            return Ok(proxy);
        }
        return parse_proxy_at_parts(trimmed);
    }

    parse_proxy_parts(trimmed)
}

fn parse_proxy_url(input: &str) -> Result<ProxySettings> {
    let url = Url::parse(input).context("Invalid proxy URL")?;
    if url.scheme() != "http" {
        anyhow::bail!("Only HTTP proxies are supported");
    }

    let host = url
        .host_str()
        .filter(|value| !value.trim().is_empty())
        .context("Proxy host is required")?
        .to_string();
    let port = url.port().context("Proxy port is required")?;
    if port == 0 {
        anyhow::bail!("Proxy port must be greater than 0");
    }
    let username = decode_url_component(url.username())?;
    let password = match url.password() {
        Some(value) => decode_url_component(value)?,
        None => None,
    };

    validate_auth_pair(&username, &password)?;

    Ok(ProxySettings {
        enabled: true,
        host,
        port,
        username,
        password,
    })
}

fn parse_proxy_parts(input: &str) -> Result<ProxySettings> {
    let parts = input.split(':').collect::<Vec<_>>();
    let (host, port, username, password) = match parts.as_slice() {
        [host, port] => ((*host).to_string(), parse_port(port)?, None, None),
        [host, port, username, password] => (
            (*host).to_string(),
            parse_port(port)?,
            Some((*username).to_string()),
            Some((*password).to_string()),
        ),
        _ => anyhow::bail!("Use host:port or host:port:username:password"),
    };

    let proxy = ProxySettings {
        enabled: true,
        host,
        port,
        username,
        password,
    };
    validate_proxy_settings(&proxy)?;
    Ok(proxy)
}

fn parse_proxy_at_parts(input: &str) -> Result<ProxySettings> {
    let parts = input.split('@').collect::<Vec<_>>();
    let [left, right] = parts.as_slice() else {
        anyhow::bail!("Use username:password@host:port or host:port@username:password");
    };

    let left_endpoint = parse_endpoint_auth_proxy(left, right);
    let right_endpoint = parse_auth_endpoint_proxy(left, right);

    match (left_endpoint, right_endpoint) {
        (Ok(proxy), Err(_)) => Ok(proxy),
        (Err(_), Ok(proxy)) => Ok(proxy),
        (Ok(left_proxy), Ok(right_proxy)) => {
            if looks_like_proxy_host(&left_proxy.host) && !looks_like_proxy_host(&right_proxy.host)
            {
                Ok(left_proxy)
            } else if looks_like_proxy_host(&right_proxy.host)
                && !looks_like_proxy_host(&left_proxy.host)
            {
                Ok(right_proxy)
            } else {
                Ok(left_proxy)
            }
        }
        (Err(left_error), Err(_)) => Err(left_error)
            .context("Use username:password@host:port or host:port@username:password"),
    }
}

fn parse_endpoint_auth_proxy(endpoint: &str, auth: &str) -> Result<ProxySettings> {
    let (host, port) = parse_host_port(endpoint)?;
    let (username, password) = parse_username_password(auth)?;
    let proxy = ProxySettings {
        enabled: true,
        host,
        port,
        username: Some(username),
        password: Some(password),
    };
    validate_proxy_settings(&proxy)?;
    Ok(proxy)
}

fn parse_auth_endpoint_proxy(auth: &str, endpoint: &str) -> Result<ProxySettings> {
    let (username, password) = parse_username_password(auth)?;
    let (host, port) = parse_host_port(endpoint)?;
    let proxy = ProxySettings {
        enabled: true,
        host,
        port,
        username: Some(username),
        password: Some(password),
    };
    validate_proxy_settings(&proxy)?;
    Ok(proxy)
}

fn parse_host_port(input: &str) -> Result<(String, u16)> {
    let (host, port) = input
        .split_once(':')
        .context("Proxy host and port must be separated by ':'")?;
    Ok((host.to_string(), parse_port(port)?))
}

fn parse_username_password(input: &str) -> Result<(String, String)> {
    let (username, password) = input
        .split_once(':')
        .context("Proxy username and password must be separated by ':'")?;
    Ok((username.to_string(), password.to_string()))
}

fn looks_like_proxy_host(value: &str) -> bool {
    let value = value.trim().trim_start_matches('[').trim_end_matches(']');
    value.eq_ignore_ascii_case("localhost")
        || value.parse::<std::net::IpAddr>().is_ok()
        || value.contains('.')
        || value.contains(':')
}

fn parse_port(value: &str) -> Result<u16> {
    let port = value
        .parse::<u16>()
        .with_context(|| format!("Invalid proxy port: {value}"))?;
    if port == 0 {
        anyhow::bail!("Proxy port must be greater than 0");
    }
    Ok(port)
}

fn decode_url_component(value: &str) -> Result<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }

    let decoded =
        urlencoding::decode(value).context("Invalid URL encoding in proxy credentials")?;
    Ok(Some(decoded.into_owned()))
}

pub fn validate_proxy_settings(proxy: &ProxySettings) -> Result<()> {
    if proxy.host.trim().is_empty() {
        anyhow::bail!("Proxy host is required");
    }
    validate_auth_pair(&proxy.username, &proxy.password)
}

fn validate_auth_pair(username: &Option<String>, password: &Option<String>) -> Result<()> {
    match (username, password) {
        (Some(username), Some(password)) if username.is_empty() || password.is_empty() => {
            anyhow::bail!("Proxy username and password cannot be empty")
        }
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => anyhow::bail!("Proxy username and password must be provided together"),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_proxy_url, parse_proxy_settings, ProxySettingsInfo};

    #[test]
    fn parses_host_port_username_password() {
        let proxy = parse_proxy_settings("192.0.2.10:8000:user:pass").unwrap();

        assert!(proxy.enabled);
        assert_eq!(proxy.host, "192.0.2.10");
        assert_eq!(proxy.port, 8000);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn parses_host_port_at_username_password_into_structured_settings() {
        let proxy = parse_proxy_settings("192.0.2.10:8000@user:pass").unwrap();

        assert!(proxy.enabled);
        assert_eq!(proxy.host, "192.0.2.10");
        assert_eq!(proxy.port, 8000);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn parses_username_password_at_host_port_into_structured_settings() {
        let proxy = parse_proxy_settings("user:pass@192.0.2.10:8000").unwrap();

        assert!(proxy.enabled);
        assert_eq!(proxy.host, "192.0.2.10");
        assert_eq!(proxy.port, 8000);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn parses_http_url_with_auth() {
        let proxy = parse_proxy_settings("http://user:pass@example.com:8080").unwrap();

        assert_eq!(proxy.host, "example.com");
        assert_eq!(proxy.port, 8080);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn parses_proxy_without_auth() {
        let proxy = parse_proxy_settings("example.com:8080").unwrap();

        assert_eq!(proxy.host, "example.com");
        assert_eq!(proxy.port, 8080);
        assert_eq!(proxy.username, None);
        assert_eq!(proxy.password, None);
    }

    #[test]
    fn rejects_invalid_proxy_inputs() {
        assert!(parse_proxy_settings(":8080").is_err());
        assert!(parse_proxy_settings("example.com:notaport").is_err());
        assert!(parse_proxy_settings("socks5://example.com:1080").is_err());
        assert!(parse_proxy_settings("http://user@example.com:8080").is_err());
    }

    #[test]
    fn proxy_info_masks_password() {
        let proxy = parse_proxy_settings("example.com:8080:user:pass").unwrap();
        let info = ProxySettingsInfo::from(Some(proxy));

        assert!(info.configured);
        assert!(info.has_password);
        assert_eq!(info.username.as_deref(), Some("user"));
    }

    #[test]
    fn builds_proxy_url_with_encoded_auth_for_environment() {
        let proxy = parse_proxy_settings("example.com:8080:user:p@ss word").unwrap();
        let url = build_proxy_url(&proxy, true).unwrap();

        assert_eq!(url, "http://user:p%40ss%20word@example.com:8080/");
    }
}
