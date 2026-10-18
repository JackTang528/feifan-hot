use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 云端更新清单地址。写在 `update_config.url`，改地址不用改代码。
/// 允许 `#` 注释和空行；只认第一条 http(s) 地址。
pub fn manifest_url() -> Option<String> {
    include_str!("../update_config.url")
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("http://") || line.starts_with("https://"))
        .map(str::to_string)
}

/// 云端 `latest.json`。只要求 `version`，其余都可缺省。
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    /// 安装包直链，点击「下载更新」用系统浏览器打开。
    #[serde(default)]
    pub url: Option<String>,
    /// 没有直链时可以给一个下载页。
    #[serde(default)]
    pub page_url: Option<String>,
    /// 更新说明。支持字符串或字符串数组（数组会按行拼接）。
    #[serde(default, deserialize_with = "de_notes")]
    pub notes: Option<String>,
    /// 发布日期，纯展示。
    #[serde(default)]
    pub published_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawNotes {
    Text(String),
    Lines(Vec<String>),
}

fn de_notes<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<RawNotes>::deserialize(deserializer)?;
    Ok(raw
        .map(|notes| match notes {
            RawNotes::Text(text) => text.trim().to_string(),
            RawNotes::Lines(lines) => lines
                .into_iter()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .filter(|text| !text.is_empty()))
}

pub fn parse(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("更新清单格式不对：{e}"))
}

/// 前端展示用的检查结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub url: Option<String>,
    pub page_url: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub manifest_url: String,
}

impl UpdateInfo {
    pub fn new(current: String, manifest: Manifest, manifest_url: String) -> Self {
        let has_update = is_newer(&manifest.version, &current);
        Self {
            current_version: current,
            latest_version: manifest.version,
            has_update,
            url: manifest.url.filter(|url| !url.trim().is_empty()),
            page_url: manifest.page_url.filter(|url| !url.trim().is_empty()),
            notes: manifest.notes,
            published_at: manifest.published_at,
            manifest_url,
        }
    }
}

/// `latest` 是否比 `current` 新。按点分数字比较，`v` 前缀和预发布后缀忽略。
pub fn is_newer(latest: &str, current: &str) -> bool {
    compare(latest, current) == Ordering::Greater
}

fn compare(a: &str, b: &str) -> Ordering {
    let parts = |version: &str| -> Vec<u64> {
        version
            .trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let a = parts(a);
    let b = parts(b);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_are_detected() {
        assert!(is_newer("0.1.3", "0.1.2"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("0.1.2.1", "0.1.2"));
    }

    #[test]
    fn same_or_older_is_not_an_update() {
        assert!(!is_newer("0.1.2", "0.1.2"));
        assert!(!is_newer("v0.1.2", "0.1.2"));
        assert!(!is_newer("0.1.1", "0.1.2"));
        assert!(!is_newer("0.1", "0.1.0"));
    }

    #[test]
    fn manifest_parses_string_notes() {
        let manifest = parse(
            r#"{"version":"0.1.4","url":"https://example.com/a.exe","notes":"修了几个问题"}"#,
        )
        .unwrap();
        assert_eq!(manifest.version, "0.1.4");
        assert_eq!(manifest.notes.as_deref(), Some("修了几个问题"));
        assert_eq!(manifest.url.as_deref(), Some("https://example.com/a.exe"));
    }

    #[test]
    fn manifest_parses_notes_array() {
        let manifest =
            parse(r#"{"version":"0.1.4","notes":["第一行","  ","第二行"]}"#).unwrap();
        assert_eq!(manifest.notes.as_deref(), Some("第一行\n第二行"));
        assert!(manifest.url.is_none());
    }

    #[test]
    fn bad_manifest_has_readable_error() {
        let err = parse("not json").unwrap_err();
        assert!(err.starts_with("更新清单格式不对"));
    }

    /// 清单地址编译进程序，盯住别改成 404 的路径。
    #[test]
    fn manifest_url_is_configured() {
        let url = manifest_url().expect("update_config.url 里要有 http(s) 地址");
        assert_eq!(url, "https://sport.tomist.com/hot-board/latest.json");
    }

    #[test]
    fn update_info_marks_update() {
        let manifest = parse(r#"{"version":"0.1.4"}"#).unwrap();
        let info = UpdateInfo::new("0.1.3".into(), manifest, "https://x/latest.json".into());
        assert!(info.has_update);
        assert_eq!(info.latest_version, "0.1.4");
    }
}
