use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use serde::Deserialize;

/// x.com 网页未登录请求使用的公开 token，不是账号密钥。
const PUBLIC_BEARER: &str =
    "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs=1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub struct X;

#[derive(Deserialize)]
struct GuestToken {
    guest_token: String,
}

#[derive(Deserialize)]
struct TrendsResponse {
    trends: Vec<Trend>,
}

#[derive(Deserialize)]
struct Trend {
    name: String,
    url: String,
    #[serde(default)]
    tweet_volume: Option<u64>,
    #[serde(default)]
    promoted_content: Option<serde_json::Value>,
}

fn x_url(url: &str) -> String {
    url.replace("http://twitter.com", "https://x.com")
        .replace("https://twitter.com", "https://x.com")
}

fn format_volume(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.1}万 帖", n as f64 / 10_000.0)
    } else {
        format!("{n} 帖")
    }
}

fn map_trends(places: &[TrendsResponse]) -> Vec<HotItem> {
    let trends = places
        .first()
        .map(|place| place.trends.as_slice())
        .unwrap_or(&[]);
    trends
        .iter()
        .filter(|trend| trend.promoted_content.is_none() && !trend.name.is_empty())
        .take(30)
        .enumerate()
        .map(|(i, trend)| HotItem {
            rank: i as u32 + 1,
            title: trend.name.clone(),
            url: x_url(&trend.url),
            hot: trend.tweet_volume.map(format_volume),
        })
        .collect()
}

#[async_trait]
impl Source for X {
    fn id(&self) -> &str {
        "x"
    }

    fn name(&self) -> &str {
        "X 热门"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let guest: GuestToken = client
            .post("https://api.twitter.com/1.1/guest/activate.json")
            .header("Authorization", format!("Bearer {PUBLIC_BEARER}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let places: Vec<TrendsResponse> = client
            .get("https://api.twitter.com/1.1/trends/place.json?id=1")
            .header("Authorization", format!("Bearer {PUBLIC_BEARER}"))
            .header("x-guest-token", guest.guest_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let items = map_trends(&places);
        if items.is_empty() {
            anyhow::bail!("X 热门没有解析到条目");
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_trends_skips_promoted_and_rewrites_url() {
        let body = r##"[
          {
            "trends": [
              {
                "name": "#topic",
                "url": "http://twitter.com/search?q=%23topic",
                "promoted_content": null,
                "tweet_volume": 20000
              },
              {
                "name": "ad",
                "url": "https://twitter.com/search?q=ad",
                "promoted_content": {},
                "tweet_volume": 1
              },
              {
                "name": "plain",
                "url": "https://x.com/search?q=plain"
              }
            ]
          }
        ]"##;
        let parsed: Vec<TrendsResponse> = serde_json::from_str(body).unwrap();
        let items = map_trends(&parsed);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "#topic");
        assert_eq!(items[0].url, "https://x.com/search?q=%23topic");
        assert_eq!(items[0].hot.as_deref(), Some("2.0万 帖"));
        assert_eq!(items[1].title, "plain");
        assert_eq!(items[1].url, "https://x.com/search?q=plain");
        assert!(items[1].hot.is_none());
    }
}
