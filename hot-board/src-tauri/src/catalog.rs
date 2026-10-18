use crate::proxy::ProxyMode;
use crate::sources::{bilibili, hackernews, rss_feed::RssFeed, toutiao, weibo, x, Source};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

const EMBEDDED: &str = include_str!("../boards.json");

#[derive(Debug, Clone, Deserialize)]
struct CatalogFile {
    boards: Vec<BoardConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardConfig {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub url: Option<String>,
    pub title_tag: Option<String>,
    pub link_tag: Option<String>,
    pub hot_tag: Option<String>,
    pub hot_suffix: Option<String>,
    pub link_template: Option<String>,
    pub split_title_source: Option<bool>,
    /// 云端给这个源配的默认代理模式。缺省时 = Auto，即跟随全局设置。
    #[serde(default, deserialize_with = "de_source_mode")]
    pub source_mode: ProxyMode,
}

/// 兼容多种写法：
///   `"sourceMode": "proxy" | "direct" | "auto"`
///   `"useProxy": true | false`
fn de_source_mode<'de, D>(deserializer: D) -> Result<ProxyMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Bool(bool),
        Str(String),
    }
    let raw = match Option::<Raw>::deserialize(deserializer)? {
        Some(raw) => raw,
        // 缺省 → Auto，跟随全局设置
        None => return Ok(ProxyMode::Auto),
    };
    match raw {
        Raw::Bool(true) => Ok(ProxyMode::Proxy),
        Raw::Bool(false) => Ok(ProxyMode::Direct),
        Raw::Str(s) => match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(ProxyMode::Auto),
            "proxy" => Ok(ProxyMode::Proxy),
            "direct" => Ok(ProxyMode::Direct),
            _ => Ok(ProxyMode::Auto),
        },
    }
}

/// 云端地址写在 `board_config.url`。空着就只用内置清单。
pub fn config_url() -> Option<String> {
    include_str!("../board_config.url").lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        if line.starts_with("https://") || line.starts_with("http://") {
            Some(line.to_string())
        } else {
            None
        }
    })
}

pub fn load_local(path: &Path) -> Vec<BoardConfig> {
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Ok(boards) = parse(&text) {
            if !boards.is_empty() {
                return boards;
            }
        }
    }
    parse(EMBEDDED).expect("内置 boards.json 无法解析")
}

/// 先拉云端；失败就沿用内存里的清单，再不行用磁盘缓存或内置清单。
pub async fn resolve(
    client: &reqwest::Client,
    path: &Path,
    current: &[BoardConfig],
) -> Vec<BoardConfig> {
    if let Some(url) = config_url() {
        if let Ok(text) = client
            .get(&url)
            .timeout(Duration::from_secs(8))
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
        {
            if let Ok(text) = text.text().await {
                if let Ok(boards) = parse(&text) {
                    if !boards.is_empty() {
                        let _ = std::fs::write(path, &text);
                        return boards;
                    }
                }
            }
        }
    }
    if !current.is_empty() {
        return current.to_vec();
    }
    load_local(path)
}

pub fn build_sources(boards: &[BoardConfig]) -> Vec<Box<dyn Source>> {
    boards
        .iter()
        .filter(|board| !board.id.trim().is_empty())
        .map(|board| named(board))
        .collect()
}

/// 从单个 `BoardConfig` 构造对应的 `Source`。空 id 返回 `Broken`。
pub fn build_source(board: &BoardConfig) -> Box<dyn Source> {
    if board.id.trim().is_empty() {
        return Box::new(Broken {
            id: board.id.clone(),
            message: "榜单 id 为空".into(),
        });
    }
    named(board)
}

fn named(board: &BoardConfig) -> Box<dyn Source> {
    let name = if board.name.trim().is_empty() {
        board.id.clone()
    } else {
        board.name.clone()
    };
    let inner = match board.kind.as_str() {
        "builtin" => builtin(&board.id).unwrap_or_else(|| {
            Box::new(Broken {
                id: board.id.clone(),
                message: format!("当前版本没有内置榜单 {}", board.id),
            })
        }),
        "rss" => rss_source(board).unwrap_or_else(|message| {
            Box::new(Broken {
                id: board.id.clone(),
                message,
            })
        }),
        other => Box::new(Broken {
            id: board.id.clone(),
            message: format!("不认识的榜单类型 {other}"),
        }),
    };
    Box::new(NamedSource { name, inner })
}

fn builtin(id: &str) -> Option<Box<dyn Source>> {
    match id {
        "hackernews" => Some(Box::new(hackernews::HackerNews)),
        "weibo" => Some(Box::new(weibo::Weibo)),
        "x" => Some(Box::new(x::X)),
        "bilibili" => Some(Box::new(bilibili::Bilibili)),
        "toutiao" => Some(Box::new(toutiao::Toutiao)),
        _ => None,
    }
}

fn rss_source(board: &BoardConfig) -> Result<Box<dyn Source>, String> {
    let url = board
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        .ok_or_else(|| format!("{} 缺少 http(s) 地址", board.name))?
        .to_string();
    Ok(Box::new(RssFeed {
        id: board.id.clone(),
        name: board.name.clone(),
        url,
        title_tag: board
            .title_tag
            .clone()
            .filter(|tag| !tag.trim().is_empty())
            .unwrap_or_else(|| "title".into()),
        link_tag: board
            .link_tag
            .clone()
            .filter(|tag| !tag.trim().is_empty())
            .unwrap_or_else(|| "link".into()),
        hot_tag: board
            .hot_tag
            .clone()
            .filter(|tag| !tag.trim().is_empty()),
        hot_suffix: board.hot_suffix.clone().unwrap_or_default(),
        link_template: board
            .link_template
            .clone()
            .filter(|template| !template.trim().is_empty()),
        split_title_source: board.split_title_source.unwrap_or(false),
    }))
}

fn parse(text: &str) -> Result<Vec<BoardConfig>, ()> {
    let file: CatalogFile = serde_json::from_str(text).map_err(|_| ())?;
    let mut seen = std::collections::HashSet::new();
    let boards = file
        .boards
        .into_iter()
        .filter(|board| seen.insert(board.id.clone()))
        .collect();
    Ok(boards)
}

struct NamedSource {
    name: String,
    inner: Box<dyn Source>,
}

#[async_trait::async_trait]
impl Source for NamedSource {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<crate::model::HotItem>> {
        self.inner.fetch(client).await
    }
}

struct Broken {
    id: String,
    message: String,
}

#[async_trait::async_trait]
impl Source for Broken {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    async fn fetch(&self, _client: &reqwest::Client) -> anyhow::Result<Vec<crate::model::HotItem>> {
        anyhow::bail!("{}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_lists_google_feeds() {
        let boards = parse(EMBEDDED).unwrap();
        assert!(boards.iter().any(|b| b.id == "google_trends" && b.kind == "rss"));
        assert!(boards.iter().any(|b| b.id == "google_news" && b.kind == "rss"));
        assert!(boards.iter().any(|b| b.id == "weibo" && b.kind == "builtin"));
        assert!(boards.iter().any(|b| b.id == "x" && b.kind == "builtin"));
        let sources = build_sources(&boards);
        assert_eq!(sources.len(), 7);
    }
}
