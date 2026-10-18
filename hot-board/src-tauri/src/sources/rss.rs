/// 从 RSS 里切出每个 `<item>…</item>`。
pub fn items(xml: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<item") {
        let after = &rest[start..];
        let Some(open_end) = after.find('>') else { break };
        let Some(close) = after[open_end..].find("</item>") else {
            break;
        };
        let end = open_end + close;
        out.push(&after[open_end + 1..end]);
        rest = &after[end + "</item>".len()..];
    }
    out
}

/// 取出某个标签的全部文本，按文档顺序。自闭合标签忽略。
pub fn texts(xml: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after_name = &rest[at + open.len()..];
        let boundary = after_name.as_bytes().first().copied();
        if !matches!(
            boundary,
            Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            rest = &rest[at + 1..];
            continue;
        }
        let Some(gt) = after_name.find('>') else { break };
        if after_name[..gt].trim_end().ends_with('/') {
            rest = &after_name[gt + 1..];
            continue;
        }
        let content = &after_name[gt + 1..];
        let Some(end) = content.find(&close) else { break };
        let text = decode_text(&content[..end]);
        let text = text.trim();
        if !text.is_empty() {
            out.push(text.to_string());
        }
        rest = &content[end + close.len()..];
    }
    out
}

pub fn text(xml: &str, tag: &str) -> Option<String> {
    texts(xml, tag).into_iter().next()
}

/// 处理标签正文：`<![CDATA[…]]>` 里的内容原样保留（不解码实体），
/// CDATA 之外的部分照常解码实体。
fn decode_text(input: &str) -> String {
    const OPEN: &str = "<![CDATA[";
    const CLOSE: &str = "]]>";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&decode_entities(&rest[..start]));
        let after = &rest[start + OPEN.len()..];
        match after.find(CLOSE) {
            Some(end) => {
                out.push_str(&after[..end]);
                rest = &after[end + CLOSE.len()..];
            }
            None => {
                // 没有结束标记：把剩余内容当作 CDATA 正文
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(&decode_entities(rest));
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        if let Some(end) = after.find(';') {
            let token = &after[..=end];
            if let Some(value) = decode_entity(token) {
                out.push_str(&value);
                rest = &after[token.len()..];
                continue;
            }
        }
        out.push('&');
        rest = &after[1..];
    }
    out.push_str(rest);
    out
}

fn decode_entity(token: &str) -> Option<String> {
    let body = &token[1..token.len() - 1];
    if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
        let n = u32::from_str_radix(hex, 16).ok()?;
        return char::from_u32(n).map(|c| c.to_string());
    }
    if let Some(num) = body.strip_prefix('#') {
        let n: u32 = num.parse().ok()?;
        return char::from_u32(n).map(|c| c.to_string());
    }
    let value = match body {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" | "ensp" | "emsp" => " ",
        "hellip" => "…",
        "mdash" => "—",
        "ndash" => "–",
        "ldquo" => "“",
        "rdquo" => "”",
        "lsquo" => "‘",
        "rsquo" => "’",
        _ => return None,
    };
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_cdata_and_keeps_literal_text() {
        let xml = r#"
            <item>
              <title><![CDATA[Saka responds to Arteta's challenge & more]]></title>
              <link>https://www.bbc.com/sport/football/articles/abc</link>
              <description><![CDATA[<b>bold</b> &amp; raw]]></description>
            </item>
            <item>
              <title>A &amp; B <![CDATA[<C>]]> D</title>
            </item>
        "#;
        let parsed = items(xml);
        assert_eq!(
            text(parsed[0], "title").as_deref(),
            Some("Saka responds to Arteta's challenge & more")
        );
        // CDATA 内部不解码实体
        assert_eq!(
            text(parsed[0], "description").as_deref(),
            Some("<b>bold</b> &amp; raw")
        );
        // CDATA 与普通文本混合
        assert_eq!(text(parsed[1], "title").as_deref(), Some("A & B <C> D"));
    }

    #[test]
    fn parses_trends_and_news_fields() {
        let xml = r#"
            <rss>
              <channel>
                <item>
                  <title>guilfoyle</title>
                  <ht:approx_traffic>5000+</ht:approx_traffic>
                  <description/>
                  <link>https://trends.google.com/trending/rss?geo=US</link>
                  <ht:news_item>
                    <ht:news_item_title>Trump Says He Is Looking</ht:news_item_title>
                    <ht:news_item_url>https://www.nytimes.com/a</ht:news_item_url>
                  </ht:news_item>
                </item>
                <item>
                  <title>Rollercoaster accident - BBC</title>
                  <link>https://news.google.com/rss/articles/abc?oc=5</link>
                  <description>&lt;a href="https://example.com"&gt;Story&lt;/a&gt; &amp; more</description>
                </item>
              </channel>
            </rss>
        "#;
        let parsed = items(xml);
        assert_eq!(parsed.len(), 2);
        assert_eq!(text(parsed[0], "title").as_deref(), Some("guilfoyle"));
        assert_eq!(text(parsed[0], "ht:approx_traffic").as_deref(), Some("5000+"));
        assert_eq!(
            texts(parsed[0], "ht:news_item_url"),
            vec!["https://www.nytimes.com/a".to_string()]
        );
        assert_eq!(
            text(parsed[1], "description").as_deref(),
            Some("<a href=\"https://example.com\">Story</a> & more")
        );
    }
}
