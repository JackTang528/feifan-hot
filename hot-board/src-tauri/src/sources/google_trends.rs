use super::Source;
use crate::model::HotItem;
use async_trait::async_trait;
use serde_json::Value;

/// Trending Now 的 Shopping。RSS `?geo=US` 会丢掉分类参数，只能拉全部分类再按 id 筛选。
/// 第 4 个请求参数必须是 0；写成 16 时接口直接返回空。分类在每条结果的第 11 项。
const SHOPPING: i64 = 16;
/// 过去 24 小时的 Shopping 经常是空的，7 天才能凑满一列。
const HOURS: i64 = 168;
const GEO: &str = "US";

pub struct GoogleTrends;

#[async_trait]
impl Source for GoogleTrends {
    fn id(&self) -> &str {
        "google_trends"
    }

    fn name(&self) -> &str {
        "Google 趋势"
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<Vec<HotItem>> {
        let inner = serde_json::json!([null, null, GEO, 0, "en-US", HOURS]);
        let freq = serde_json::json!([[["i0OFE", inner.to_string(), null, "generic"]]]);
        let mut dummy = reqwest::Url::parse("https://example.invalid/").expect("form base");
        dummy.query_pairs_mut().append_pair("f.req", &freq.to_string());
        let body = dummy.query().unwrap_or("").to_string();

        let text = client
            .post("https://trends.google.com/_/TrendsUi/data/batchexecute?rpcids=i0OFE&source-path=%2Ftrending&hl=en-US")
            .header(
                "content-type",
                "application/x-www-form-urlencoded;charset=UTF-8",
            )
            .header("origin", "https://trends.google.com")
            .header("referer", "https://trends.google.com/trending?geo=US")
            .body(body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let items = map_rows(&trending_rows(&text)?);
        if items.is_empty() {
            anyhow::bail!("Google 购物趋势没有条目");
        }
        Ok(items)
    }
}

fn trending_rows(text: &str) -> anyhow::Result<Vec<Value>> {
    let text = text.trim_start_matches(")]}'");
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(entries) = value.as_array() else {
            continue;
        };
        for entry in entries {
            let Some(entry) = entry.as_array() else {
                continue;
            };
            if entry.first().and_then(Value::as_str) != Some("wrb.fr") {
                continue;
            }
            if entry.get(1).and_then(Value::as_str) != Some("i0OFE") {
                continue;
            }
            let Some(payload) = entry.get(2).and_then(Value::as_str) else {
                continue;
            };
            let parsed: Value = serde_json::from_str(payload)?;
            return Ok(parsed
                .get(1)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default());
        }
    }
    anyhow::bail!("Google 趋势没有返回榜单")
}

/// 只留 Shopping（分类 16），按搜索量从高到低，最多 30 条。
pub fn map_rows(rows: &[Value]) -> Vec<HotItem> {
    let mut picked = Vec::new();
    for row in rows {
        let Some(fields) = row.as_array() else {
            continue;
        };
        let title = fields
            .first()
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if title.is_empty() {
            continue;
        }
        let shopping = fields
            .get(10)
            .and_then(Value::as_array)
            .is_some_and(|cats| cats.iter().any(|cat| cat.as_i64() == Some(SHOPPING)));
        if !shopping {
            continue;
        }
        let volume = fields.get(6).and_then(Value::as_i64).unwrap_or(0);
        picked.push((volume, title.to_string()));
    }
    picked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    picked.truncate(30);
    picked
        .into_iter()
        .enumerate()
        .map(|(i, (volume, title))| HotItem {
            rank: i as u32 + 1,
            url: explore_url(&title),
            hot: (volume > 0).then(|| format!("{volume}+ 热度")),
            title,
        })
        .collect()
}

fn explore_url(title: &str) -> String {
    let mut url = reqwest::Url::parse("https://trends.google.com/trends/explore").expect("explore");
    url.query_pairs_mut().append_pair("geo", GEO).append_pair("q", title);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_shopping_and_ranks_by_volume() {
        let rows = serde_json::json!([
            ["ole miss vs texas", null, "US", [1], null, null, 200000, null, 1000, ["ole miss"], [17], [], "ole miss"],
            ["g shock", null, "US", [1], null, null, 500, null, 300, ["g shock"], [16], [], "g shock"],
            ["walmart plus", null, "US", [1], null, null, 100000, null, 75, ["walmart plus"], [16, 3], [], "walmart plus"]
        ]);
        let items = map_rows(rows.as_array().unwrap());
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "walmart plus");
        assert_eq!(items[0].hot.as_deref(), Some("100000+ 热度"));
        assert_eq!(
            items[0].url,
            "https://trends.google.com/trends/explore?geo=US&q=walmart+plus"
        );
        assert_eq!(items[1].title, "g shock");
        assert_eq!(items[1].rank, 2);
    }

    #[test]
    fn reads_shopping_rows_from_batchexecute() {
        let text = ")]}'\n123\n[[\"wrb.fr\",\"i0OFE\",\"[null,[[\\\"walmart plus\\\",null,\\\"US\\\",[1],null,null,100000,null,75,[\\\"walmart plus\\\"],[16],[],\\\"walmart plus\\\"]]]\",null]]\n";
        let rows = trending_rows(text).unwrap();
        let items = map_rows(&rows);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "walmart plus");
    }
}
