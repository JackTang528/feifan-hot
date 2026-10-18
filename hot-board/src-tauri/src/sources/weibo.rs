use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use serde::Deserialize;

pub struct Weibo;

#[derive(Deserialize)]
struct HotSearchResponse {
    data: HotSearchData,
}

#[derive(Deserialize)]
struct HotSearchData {
    realtime: Vec<HotWord>,
}

#[derive(Deserialize)]
struct HotWord {
    word: String,
    word_scheme: Option<String>,
    num: Option<u64>,
    is_ad: Option<i64>,
}

fn search_url(query: &str) -> String {
    let mut url = reqwest::Url::parse("https://s.weibo.com/weibo").expect("weibo search url");
    url.query_pairs_mut().append_pair("q", query);
    url.to_string()
}

fn format_hot(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.0}万", n as f64 / 10_000.0)
    } else {
        n.to_string()
    }
}

#[async_trait]
impl Source for Weibo {
    fn id(&self) -> &str {
        "weibo"
    }

    fn name(&self) -> &str {
        "微博热搜"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let body: HotSearchResponse = client
            .get("https://weibo.com/ajax/side/hotSearch")
            .header("Referer", "https://weibo.com/")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(body
            .data
            .realtime
            .into_iter()
            .filter(|item| item.is_ad.unwrap_or(0) == 0 && !item.word.is_empty())
            .take(30)
            .enumerate()
            .map(|(i, item)| {
                let query = item
                    .word_scheme
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("#{}#", item.word));
                HotItem {
                    rank: i as u32 + 1,
                    title: item.word,
                    url: search_url(&query),
                    hot: item.num.map(format_hot),
                }
            })
            .collect())
    }
}
