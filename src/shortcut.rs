use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Value};

/// Expand shortcut syntax in a header value string.
pub fn expand_value(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("basic!") {
        return format!("Basic {}", STANDARD.encode(rest.as_bytes()));
    }
    if let Some(rest) = s.strip_prefix("bearer!") {
        return format!("Bearer {rest}");
    }

    match s {
        "json!" | "j!" => return "application/json".to_string(),
        "form!" | "f!" => return "application/x-www-form-urlencoded".to_string(),
        "multi!" | "m!" => return "multipart/form-data".to_string(),
        "html!" | "h!" => return "text/html".to_string(),
        "text!" | "t!" => return "text/plain".to_string(),
        "xml!" | "x!" => return "application/xml".to_string(),
        _ => {}
    }

    if let Some(rest) = s.strip_prefix("a!/") {
        return format!("application/{rest}");
    }
    if let Some(rest) = s.strip_prefix("t!/") {
        return format!("text/{rest}");
    }
    if let Some(rest) = s.strip_prefix("i!/") {
        return format!("image/{rest}");
    }

    s.to_string()
}

fn expand_key(key: &str) -> Option<&'static str> {
    match key {
        "a!" | "auth!" => Some("Authorization"),
        "c!" | "ct!" => Some("Content-Type"),
        _ => None,
    }
}

/// Expand an auth value based on its type.
fn expand_auth_value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            let expanded = expand_value(s);
            if expanded != *s {
                Some(expanded)
            } else if s.contains(' ') {
                Some(s.clone())
            } else {
                Some(format!("Bearer {s}"))
            }
        }
        Value::Array(arr) if arr.len() == 2 => {
            let user = arr[0].as_str()?;
            let pass = arr[1].as_str()?;
            let credentials = format!("{user}:{pass}");
            Some(format!("Basic {}", STANDARD.encode(credentials.as_bytes())))
        }
        _ => None,
    }
}

/// Expand shortcut keys and values in a headers map.
pub fn expand_headers(headers: &mut Map<String, Value>) {
    let expansions: Vec<(String, String, Value)> = headers
        .iter()
        .filter_map(|(k, v)| {
            expand_key(k).map(|full| (k.clone(), full.to_string(), v.clone()))
        })
        .collect();

    for (old_key, new_key, val) in expansions {
        headers.remove(&old_key);
        if new_key == "Authorization" {
            if let Some(auth_val) = expand_auth_value(&val) {
                headers.insert(new_key, Value::String(auth_val));
            } else {
                headers.insert(new_key, val);
            }
        } else {
            headers.insert(new_key, val);
        }
    }

    for (k, v) in headers.iter_mut() {
        if k == "Authorization" {
            continue;
        }
        if let Value::String(s) = v {
            let expanded = expand_value(s);
            if expanded != *s {
                *v = Value::String(expanded);
            }
        }
    }
}
