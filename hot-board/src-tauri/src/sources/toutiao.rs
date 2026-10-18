use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use serde::Deserialize;

pub struct Toutiao;

#[derive(Deserialize)]
struct HotBoardResponse {
    data: Vec<HotEvent>,
}

#[derive(Deserialize)]
struct HotEvent {
    Title: String,
    Url: String,
    HotValue: Option<String>,
}

#[async_trait]
impl Source for Toutiao {
    fn id(&self) -> &str {
        "toutiao"
    }

    fn name(&self) -> &str {
        "今日头条热榜"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let body: HotBoardResponse = client
            .get("https://www.toutiao.com/hot-event/hot-board/?origin=toutiao_pc")
            .send()
            .await?
            .json()
            .await?;

        Ok(body
            .data
            .into_iter()
            .take(30)
            .enumerate()
            .map(|(i, e)| HotItem {
                rank: i as u32 + 1,
                title: e.Title,
                url: e.Url,
                hot: e.HotValue,
            })
            .collect())
    }
}
