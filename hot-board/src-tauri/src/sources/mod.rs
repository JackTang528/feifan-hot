use crate::model::HotItem;
use async_trait::async_trait;

pub mod bilibili;
pub mod google_trends;
pub mod hackernews;
pub mod rss;
pub mod rss_feed;
pub mod toutiao;
pub mod weibo;
pub mod x;

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>>;
}

