use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;

pub struct HackerNews;

#[derive(Deserialize)]
struct Story {
    title: String,
    url: Option<String>,
    id: u64,
    score: Option<u32>,
}

#[async_trait]
impl Source for HackerNews {
    fn id(&self) -> &str {
        "hackernews"
    }

    fn name(&self) -> &str {
        "Hacker News"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let ids: Vec<u64> = client
            .get("https://hacker-news.firebaseio.com/v0/topstories.json")
            .send()
            .await?
            .json()
            .await?;

        let futs = ids.into_iter().take(30).map(|id| {
            let client = client.clone();
            async move {
                client
                    .get(format!(
                        "https://hacker-news.firebaseio.com/v0/item/{id}.json"
                    ))
                    .send()
                    .await?
                    .json::<Story>()
                    .await
            }
        });

        let items = join_all(futs)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .enumerate()
            .map(|(i, s)| HotItem {
                rank: i as u32 + 1,
                url: s
                    .url
                    .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={}", s.id)),
                title: s.title,
                hot: s.score.map(|n| format!("{n} 分")),
            })
            .collect();
        Ok(items)
    }
}
