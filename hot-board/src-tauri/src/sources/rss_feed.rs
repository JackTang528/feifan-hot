use super::{rss, Source};
use crate::model::HotItem;
use async_trait::async_trait;

/// 云端清单里的一条 RSS 榜。换地址、改字段不用发新版本。
#[derive(Clone)]
pub struct RssFeed {
    pub id: String,
    pub name: String,
    pub url: String,
    pub title_tag: String,
    pub link_tag: String,
    pub hot_tag: Option<String>,
    pub hot_suffix: String,
    pub link_template: Option<String>,
    pub split_title_source: bool,
}

#[async_trait]
impl Source for RssFeed {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let xml = client
            .get(&self.url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let items = map_items(&xml, self);
        if items.is_empty() {
            anyhow::bail!("{} 没有解析到条目", self.name);
        }
        Ok(items)
    }
}

pub fn map_items(xml: &str, feed: &RssFeed) -> Vec<HotItem> {
    rss::items(xml)
        .into_iter()
        .filter_map(|item| {
            let raw_title = rss::text(item, &feed.title_tag)?;
            if raw_title.is_empty() {
                return None;
            }
            let (title, source) = if feed.split_title_source {
                split_source(&raw_title)
            } else {
                (raw_title, None)
            };
            let linked = rss::texts(item, &feed.link_tag)
                .into_iter()
                .find(|url| url.starts_with("http://") || url.starts_with("https://"));
            let url = linked.or_else(|| {
                feed.link_template
                    .as_deref()
                    .map(|template| fill_template(template, &title))
            })?;
            let hot = feed
                .hot_tag
                .as_deref()
                .and_then(|tag| rss::text(item, tag))
                .map(|value| format!("{value}{}", feed.hot_suffix))
                .or(source);
            Some(HotItem {
                rank: 0,
                title,
                url,
                hot,
            })
        })
        .take(30)
        .enumerate()
        .map(|(i, mut item)| {
            item.rank = i as u32 + 1;
            item
        })
        .collect()
}

fn split_source(title: &str) -> (String, Option<String>) {
    let Some((headline, source)) = title.rsplit_once(" - ") else {
        return (title.to_string(), None);
    };
    let headline = headline.trim();
    let source = source.trim();
    if headline.is_empty() || source.is_empty() || source.chars().count() > 40 {
        return (title.to_string(), None);
    }
    (headline.to_string(), Some(source.to_string()))
}

fn fill_template(template: &str, title: &str) -> String {
    if !template.contains("{title}") {
        return template.to_string();
    }
    let mut dummy = reqwest::Url::parse("https://example.invalid/").expect("template base");
    dummy.query_pairs_mut().append_pair("q", title);
    let encoded = dummy
        .query()
        .unwrap_or("")
        .strip_prefix("q=")
        .unwrap_or("")
        .to_string();
    template.replace("{title}", &encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trends() -> RssFeed {
        RssFeed {
            id: "google_trends".into(),
            name: "Google 趋势".into(),
            url: "https://trends.google.com/trending/rss?geo=US".into(),
            title_tag: "title".into(),
            link_tag: "ht:news_item_url".into(),
            hot_tag: Some("ht:approx_traffic".into()),
            hot_suffix: " 热度".into(),
            link_template: Some(
                "https://trends.google.com/trends/explore?geo=US&q={title}".into(),
            ),
            split_title_source: false,
        }
    }

    #[test]
    fn maps_trends_item_to_news_url_and_traffic() {
        let xml = r#"
            <item>
              <title>guilfoyle</title>
              <ht:approx_traffic>5000+</ht:approx_traffic>
              <ht:news_item_url>https://www.nytimes.com/a</ht:news_item_url>
            </item>
        "#;
        let items = map_items(xml, &trends());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "guilfoyle");
        assert_eq!(items[0].url, "https://www.nytimes.com/a");
        assert_eq!(items[0].hot.as_deref(), Some("5000+ 热度"));
    }

    #[test]
    fn news_title_splits_publisher() {
        let xml = r#"
            <item>
              <title>Rollercoaster accident - BBC</title>
              <link>https://news.google.com/rss/articles/abc</link>
            </item>
        "#;
        let feed = RssFeed {
            id: "google_news".into(),
            name: "Google 新闻".into(),
            url: "https://news.google.com/rss".into(),
            title_tag: "title".into(),
            link_tag: "link".into(),
            hot_tag: None,
            hot_suffix: String::new(),
            link_template: None,
            split_title_source: true,
        };
        let items = map_items(xml, &feed);
        assert_eq!(items[0].title, "Rollercoaster accident");
        assert_eq!(items[0].hot.as_deref(), Some("BBC"));
        assert_eq!(items[0].url, "https://news.google.com/rss/articles/abc");
    }
}
