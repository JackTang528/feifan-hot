use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use serde::Deserialize;

pub struct Bilibili;

#[derive(Deserialize)]
struct PopularResponse {
    data: PopularData,
}

#[derive(Deserialize)]
struct PopularData {
    list: Vec<PopularItem>,
}

#[derive(Deserialize)]
struct PopularItem {
    title: String,
    short_link_v2: Option<String>,
    bvid: String,
    stat: ItemStat,
}

#[derive(Deserialize)]
struct ItemStat {
    view: u64,
}

fn format_views(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.1}万播放", n as f64 / 10_000.0)
    } else {
        format!("{n} 播放")
    }
}

#[async_trait]
impl Source for Bilibili {
    fn id(&self) -> &str {
        "bilibili"
    }

    fn name(&self) -> &str {
        "B站热门"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let body: PopularResponse = client
            .get("https://api.bilibili.com/x/web-interface/popular?ps=30&pn=1")
            .send()
            .await?
            .json()
            .await?;

        Ok(body
            .data
            .list
            .into_iter()
            .enumerate()
            .map(|(i, v)| HotItem {
                rank: i as u32 + 1,
                title: v.title,
                url: v
                    .short_link_v2
                    .unwrap_or_else(|| format!("https://www.bilibili.com/video/{}", v.bvid)),
                hot: Some(format_views(v.stat.view)),
            })
            .collect())
    }
}
