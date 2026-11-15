use serde_json::Value;

/// 把各种来源的 cookie 统一成可直接放进 `Cookie` 请求头的 `name=value; name2=value2` 形式。
///
/// 支持：
/// - 浏览器扩展（Cookie-Editor / EditThisCookie）导出的 JSON 数组
/// - `{"name": "value", ...}` 形式的 JSON 对象
/// - Netscape cookies.txt（制表符分隔）
/// - 直接粘进来的 `Cookie: xxx` 请求头
/// - 仓库里 `api_client` 拿到的原始 set-cookie 串
pub fn normalize(input: &str) -> String {
    let raw = input.trim();
    if raw.is_empty() {
        return String::new();
    }

    if let Some(json) = normalize_json(raw) {
        return json;
    }
    if raw.contains('\t') {
        let netscape = normalize_netscape(raw);
        if !netscape.is_empty() {
            return netscape;
        }
    }
    normalize_header(raw)
}

fn normalize_json(raw: &str) -> Option<String> {
    let first = raw.chars().next()?;
    if first != '[' && first != '{' {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;
    let mut pairs: Vec<String> = Vec::new();
    match value {
        Value::Array(items) => {
            for item in items {
                let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
                let value = item.get("value").and_then(Value::as_str).unwrap_or_default();
                if !name.is_empty() {
                    push_pair(&mut pairs, name, value);
                }
            }
        }
        Value::Object(map) => {
            for (name, value) in map {
                let value = match value {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                if !name.is_empty() {
                    push_pair(&mut pairs, &name, &value);
                }
            }
        }
        _ => return None,
    }
    if pairs.is_empty() {
        None
    } else {
        Some(pairs.join("; "))
    }
}

fn normalize_netscape(raw: &str) -> String {
    let mut pairs: Vec<String> = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() >= 7 {
            push_pair(&mut pairs, fields[5].trim(), fields[6].trim());
        }
    }
    pairs.join("; ")
}

fn normalize_header(raw: &str) -> String {
    let mut body = raw;
    // 去掉 `Cookie:` 前缀（可能有多行）
    if let Some(pos) = body.to_ascii_lowercase().find("cookie:") {
        body = &body[pos + "cookie:".len()..];
    }
    let mut pairs: Vec<String> = Vec::new();
    for piece in body.split(&[';', '\n', '\r'][..]) {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let Some((name, value)) = piece.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        push_pair(&mut pairs, name, value.trim());
    }
    pairs.join("; ")
}

fn push_pair(pairs: &mut Vec<String>, name: &str, value: &str) {
    // 同名 cookie 以最后一次为准
    if let Some(exist) = pairs
        .iter_mut()
        .find(|pair| pair.starts_with(&format!("{name}=")))
    {
        *exist = format!("{name}={value}");
        return;
    }
    pairs.push(format!("{name}={value}"));
}

/// 从一段可能包含多行的 `set-cookie` 响应头里拼出 `name=value; ...`
pub fn from_set_cookie_headers<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut pairs: Vec<String> = Vec::new();
    for value in values {
        let Some(first) = value.split(';').next() else {
            continue;
        };
        let Some((name, val)) = first.split_once('=') else {
            continue;
        };
        if name.trim().is_empty() {
            continue;
        }
        push_pair(&mut pairs, name.trim(), val.trim());
    }
    pairs.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_array() {
        let raw = r#"[{"name":"a","value":"1"},{"name":"b","value":"2"}]"#;
        assert_eq!(normalize(raw), "a=1; b=2");
    }

    #[test]
    fn plain_header() {
        assert_eq!(normalize("Cookie: a=1; b=2;\n"), "a=1; b=2");
    }

    #[test]
    fn netscape_txt() {
        let raw = "# Netscape HTTP Cookie File\n.wnacg.com\tTRUE\t/\tTRUE\t1826277047\t_ym_uid\t175562317\n";
        assert_eq!(normalize(raw), "_ym_uid=175562317");
    }

    #[test]
    fn last_value_wins() {
        assert_eq!(normalize("a=1; a=2"), "a=2");
    }

    #[test]
    fn set_cookie_headers_are_joined() {
        let headers = ["sid=abc; Path=/; HttpOnly", "uid=42; Path=/"];
        assert_eq!(
            from_set_cookie_headers(headers.iter().copied()),
            "sid=abc; uid=42"
        );
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(normalize("   \n "), "");
        assert_eq!(normalize("[]"), "");
    }
}
