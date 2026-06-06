pub fn extract_kv<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let line = line.split('#').next()?.trim();
    let rest = line.strip_prefix(key)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    Some(rest.trim())
}

pub fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.trim_end_matches(',').trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_array(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0;
    let mut in_str: Option<char> = None;
    for ch in inner.chars() {
        match (ch, in_str) {
            ('"', None) | ('\'', None) => {
                in_str = Some(ch);
                buf.push(ch);
            }
            (c, Some(q)) if c == q => {
                in_str = None;
                buf.push(c);
            }
            ('{', None) | ('[', None) => {
                depth += 1;
                buf.push(ch);
            }
            ('}', None) | (']', None) => {
                depth -= 1;
                buf.push(ch);
            }
            (',', None) if depth == 0 => {
                let item = unquote(buf.trim());
                if !item.is_empty() {
                    out.push(item);
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    let last = unquote(buf.trim());
    if !last.is_empty() {
        out.push(last);
    }
    out
}

pub fn json_string_field(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let idx = s.find(&needle)?;
    let after = &s[idx + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let body = &rest[1..];
    let end = body.find('"')?;
    Some(body[..end].to_string())
}

pub fn json_array_field(s: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{}\"", key);
    let idx = s.find(&needle)?;
    let after = &s[idx + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    let end = rest.find(']')?;
    let inner = &rest[1..end];
    let mut out = Vec::new();
    for part in inner.split(',') {
        let p = part.trim().trim_matches('"');
        if !p.is_empty() {
            out.push(p.to_string());
        }
    }
    Some(out)
}
