//! 代理设置：软件自己读 Windows 系统代理、支持手动覆盖、每个源单独开关。
//!
//! 两层开关：
//!
//!   全局 `setting`
//!     - `Auto`   ：启动时读 Windows 系统代理（WinHTTP → WinINET → 环境变量），跟随系统变化
//!     - `Manual` ：用 `proxy_url`（设置面板里手动填的地址）
//!     - `Off`    ：一律直连，优先级最高
//!
//!   每个源的 `source_modes[source_id]`
//!     - `Auto`   ：跟随全局（云端 boards.json 里可能已经把某个源标成默认 Proxy / Direct）
//!     - `Proxy`  ：强制走代理
//!     - `Direct` ：强制直连

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SETTINGS_FILE: &str = "proxy_config.json";

// ---------------------------------------------------------------------------
// 枚举
// ---------------------------------------------------------------------------

/// 全局代理设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxySetting {
    /// 自动跟随 Windows 系统代理。
    #[default]
    Auto,
    /// 使用手动指定的代理地址（`proxy_url`）。
    Manual,
    /// 完全不走代理。
    Off,
}

impl<'de> Deserialize<'de> for ProxySetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "off" => Ok(Self::Off),
            _ => Err(serde::de::Error::custom(format!(
                "未知的代理模式 '{value}'，只支持 auto / manual / off"
            ))),
        }
    }
}

/// 每个源单独的模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxyMode {
    /// 跟随全局设置（或云端 boards.json 里配置的默认）。
    Auto,
    /// 强制走代理。
    Proxy,
    /// 强制直连。
    Direct,
}

impl Default for ProxyMode {
    fn default() -> Self {
        Self::Auto
    }
}

// ---------------------------------------------------------------------------
// 持久化配置
// ---------------------------------------------------------------------------

/// 用户持久化到 `proxy_config.json` 的设置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxySettings {
    pub setting: ProxySetting,
    /// 手动代理地址（仅 `setting == Manual` 时生效）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// 每个源的独立模式。缺省的源 = Auto（跟随全局）。
    #[serde(default)]
    pub source_modes: std::collections::BTreeMap<String, ProxyMode>,
}

impl ProxySettings {
    /// 计算某个源实际是否走代理。
    ///
    /// `server_default` = 云端 boards.json 里该源的默认模式。
    ///
    /// 优先级：
    ///   1. 全局 `Off`            → 直连
    ///   2. 本地源覆盖            → 用本地
    ///   3. 云端 boards.json 默认 → 用云端
    ///   4. 全局 `Manual`         → 走 proxy_url
    ///   5. 全局 `Auto`           → 跟随 Windows 系统代理
    pub fn use_proxy(&self, source_id: &str, server_default: ProxyMode) -> bool {
        if matches!(self.setting, ProxySetting::Off) {
            return false;
        }
        let mode = self.effective_mode(source_id, server_default);
        match mode {
            ProxyMode::Proxy => true,
            ProxyMode::Direct => false,
            ProxyMode::Auto => match self.setting {
                ProxySetting::Manual => true,
                ProxySetting::Auto => detect_system_proxy().is_some(),
                ProxySetting::Off => false,
            },
        }
    }

    /// 计算某个源实际的模式（合并三层）。
    pub fn effective_mode(&self, source_id: &str, server_default: ProxyMode) -> ProxyMode {
        self.source_modes
            .get(source_id)
            .copied()
            .unwrap_or(server_default)
    }
}

pub fn load_settings(path: &Path) -> ProxySettings {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => ProxySettings::default(),
    }
}

pub fn write_settings(settings: &ProxySettings, path: &Path) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(settings).map_err(|e| format!("序列化设置失败: {e}"))?;
    std::fs::write(path, text).map_err(|e| format!("写入 {SETTINGS_FILE} 失败: {e}"))
}

// ---------------------------------------------------------------------------
// 系统代理检测
// ---------------------------------------------------------------------------

/// 读取系统代理设置并规范化成 `http://host:port[?no_proxy=...]` 形式。
pub fn detect_system_proxy() -> Option<String> {
    #[cfg(windows)]
    {
        if let Some(url) = system_windows() {
            return Some(url);
        }
    }
    system_env_proxy()
}

#[cfg(windows)]
fn system_windows() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const WINHTTP: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings\WinHttp";
    const WININET: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // WinHTTP 优先（大部分 Windows 客户端包括 curl / git / Tauri 走这个）
    // WinINET 次之（浏览器 / IE / Edge 走这个）
    let subkeys = [WINHTTP, WININET];
    for subkey in subkeys {
        let Ok(key) = hkcu.open_subkey(subkey) else {
            continue;
        };
        let Ok(server) = key.get_value::<String, _>("ProxyServer") else {
            continue;
        };
        // 代理绕过列表（bypass）在两个位置都存在，WinHTTP 通常没有该键
        let bypass = key.get_value::<String, _>("ProxyOverride").ok();
        if let Some(url) = build_proxy_url(&server, bypass.as_deref()) {
            return Some(url);
        }
    }
    None
}

/// 把 Windows 注册表里 `ProxyServer` 的值解析成 `http://host:port[?no_proxy=...]`。
fn build_proxy_url(server: &str, bypass: Option<&str>) -> Option<String> {
    let entry = server
        .split([';', ' ', '\t', '\n', '\r'])
        .find(|s| !s.trim().is_empty())?;
    let entry = entry.trim();

    // 去掉可能的协议前缀，只保留 `host:port`
    let address = match split_scheme(entry) {
        Some(addr) if !addr.trim().is_empty() => addr.trim(),
        _ => entry,
    };
    if address.is_empty() {
        return None;
    }

    // 组装成 `http://host:port`
    let mut url = format!("http://{address}");
    if let Some(bypass) = bypass {
        let list = bypass
            .split([';', ' ', '\t', '\n', '\r'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");
        if !list.is_empty() {
            let encoded = percent_encode_list(&list);
            url.push_str(&format!("?no_proxy={encoded}"));
        }
    }
    // 校验一下最终是合法 URL
    url::Url::parse(&url).ok()?;
    Some(url)
}

/// 简单地把逗号分隔的绕过列表做 URL 编码。
fn percent_encode_list(list: &str) -> String {
    let mut out = String::with_capacity(list.len());
    for c in list.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '\t' => out.push_str("%09"),
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' | ',' | '*' | ';' | '<' | '>' => {
                out.push(c)
            }
            _ => out.push_str(&format!("%{:02X}", c as u32)),
        }
    }
    out
}

/// 如果输入包含协议前缀，去掉它，返回纯 `host:port`。
fn split_scheme(input: &str) -> Option<&str> {
    let (_scheme, rest) = input.split_once("://")?;
    Some(rest)
}

/// 非 Windows 平台 / Windows 兜底：读 `HTTP(S)_PROXY` 环境变量。
fn system_env_proxy() -> Option<String> {
    const KEYS: [&str; 2] = ["HTTPS_PROXY", "HTTP_PROXY"];
    for key in KEYS {
        if let Some(value) = std::env::var_os(key).and_then(|v| v.into_string().ok()) {
            let value = value.trim().to_string();
            if value.is_empty() {
                continue;
            }
            if value.contains("://") {
                return Some(value);
            }
            // 补上 `http://` 前缀
            return Some(format!("http://{value}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_setting_is_auto() {
        let settings = ProxySettings::default();
        assert_eq!(settings.setting, ProxySetting::Auto);
        assert!(settings.proxy_url.is_none());
        assert!(settings.source_modes.is_empty());
    }

    #[test]
    fn manual_setting_makes_auto_sources_use_proxy() {
        let mut settings = ProxySettings::default();
        settings.setting = ProxySetting::Manual;
        settings.proxy_url = Some("http://127.0.0.1:7897".into());
        assert!(settings.use_proxy("any", ProxyMode::Auto));
        assert!(settings.use_proxy("google_trends", ProxyMode::Auto));
    }

    #[test]
    fn source_override_direct_wins_over_global_manual() {
        let mut settings = ProxySettings::default();
        settings.setting = ProxySetting::Manual;
        settings.proxy_url = Some("http://127.0.0.1:7897".into());
        settings.source_modes.insert("weibo".into(), ProxyMode::Direct);
        assert!(settings.use_proxy("google_trends", ProxyMode::Auto));
        assert!(!settings.use_proxy("weibo", ProxyMode::Auto));
    }

    #[test]
    fn global_off_forces_direct_even_when_source_says_proxy() {
        let mut settings = ProxySettings {
            setting: ProxySetting::Off,
            ..Default::default()
        };
        settings.source_modes.insert("google_trends".into(), ProxyMode::Proxy);
        assert!(!settings.use_proxy("google_trends", ProxyMode::Proxy));
        assert!(!settings.use_proxy("weibo", ProxyMode::Direct));
    }

    #[test]
    fn source_override_proxy_overrides_server_default_direct() {
        let mut settings = ProxySettings::default();
        settings.source_modes.insert("weibo".into(), ProxyMode::Proxy);
        assert!(settings.use_proxy("weibo", ProxyMode::Direct));
    }

    #[test]
    fn server_default_direct_wins_when_no_user_override_and_global_auto() {
        let settings = ProxySettings::default();
        assert!(!settings.use_proxy("weibo", ProxyMode::Direct));
    }

    #[test]
    fn server_default_proxy_uses_proxy_when_global_auto() {
        // 云端标 Proxy + 全局 Auto → 返回 true（调用方负责降级）
        let settings = ProxySettings::default();
        assert!(settings.use_proxy("google_trends", ProxyMode::Proxy));
    }

    #[test]
    fn effective_mode_resolves_layers() {
        let mut settings = ProxySettings::default();
        settings.source_modes.insert("a".into(), ProxyMode::Proxy);
        assert_eq!(settings.effective_mode("a", ProxyMode::Direct), ProxyMode::Proxy);
        assert_eq!(settings.effective_mode("b", ProxyMode::Direct), ProxyMode::Direct);
        assert_eq!(settings.effective_mode("c", ProxyMode::Proxy), ProxyMode::Proxy);
    }

    #[test]
    fn build_proxy_url_strips_scheme_and_uses_http() {
        let url = build_proxy_url("127.0.0.1:7897", None).unwrap();
        assert_eq!(url, "http://127.0.0.1:7897");

        let url = build_proxy_url("http://127.0.0.1:7897", None).unwrap();
        assert_eq!(url, "http://127.0.0.1:7897");
    }

    #[test]
    fn build_proxy_url_appends_no_proxy_from_bypass() {
        let url = build_proxy_url("127.0.0.1:7897", Some("localhost;127.*;<local>")).unwrap();
        assert!(url.starts_with("http://127.0.0.1:7897?no_proxy="));
        assert!(url.contains("localhost"));
        assert!(url.contains("127.*"));
    }

    #[test]
    fn build_proxy_url_picks_first_entry_when_semicolon_separated() {
        let url = build_proxy_url("socks5://127.0.0.1:1080;http://127.0.0.1:7897", None)
            .unwrap();
        assert!(url.starts_with("http://127.0.0.1:1080"));
    }

    #[test]
    fn build_proxy_url_rejects_empty() {
        assert!(build_proxy_url("", None).is_none());
        assert!(build_proxy_url("   ;   ", None).is_none());
    }
}
