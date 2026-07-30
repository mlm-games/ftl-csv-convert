use std::collections::BTreeMap;
use std::path::Path;

pub type Messages = BTreeMap<String, String>;
pub type Contexts = BTreeMap<String, String>;

pub struct ParsedFtl {
    pub messages: Messages,
    pub contexts: Contexts,
}

pub fn is_valid_ftl_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn parse_ftl_file(content: &str, path: &Path) -> ParsedFtl {
    let mut messages = Messages::new();
    let mut contexts = Contexts::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut pending_comments: Vec<String> = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            pending_comments.clear();
            i += 1;
            continue;
        }

        if trimmed.starts_with('#') {
            let text = trimmed.trim_start_matches('#').trim_start();
            pending_comments.push(text.to_string());
            i += 1;
            continue;
        }

        if trimmed.starts_with('-') {
            eprintln!(
                "warning: skipping term in {}: {}",
                path.display(),
                truncate(trimmed, 50)
            );
            pending_comments.clear();
            i = skip_entry_body(&lines, i);
            continue;
        }

        if trimmed.starts_with('.') {
            eprintln!(
                "warning: skipping attribute line in {}: {}",
                path.display(),
                truncate(trimmed, 50)
            );
            pending_comments.clear();
            i += 1;
            continue;
        }

        if let Some(eq) = find_top_level_eq(trimmed) {
            let key = trimmed[..eq].trim();
            if !is_valid_ftl_id(key) {
                eprintln!(
                    "warning: skipping invalid FTL id '{key}' in {}",
                    path.display()
                );
                pending_comments.clear();
                i = skip_entry_body(&lines, i);
                continue;
            }

            let after = trimmed[eq + 1..].trim_start();
            let value = if after.is_empty() {
                let (val, next) = read_multiline_value(&lines, i + 1);
                i = next;
                val
            } else {
                i += 1;
                after.to_string()
            };

            if value_looks_like_selector(&value) {
                eprintln!(
                    "warning: key '{key}' looks like a selector; storing raw text from {}",
                    path.display()
                );
            }

            if !pending_comments.is_empty() {
                let ctx = pending_comments
                    .iter()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");
                if !ctx.is_empty() {
                    contexts.insert(key.to_string(), ctx);
                }
                pending_comments.clear();
            }

            while i < lines.len() {
                let t = lines[i].trim();
                if t.starts_with('.') {
                    eprintln!(
                        "warning: skipping attribute on '{key}' in {}",
                        path.display()
                    );
                    i += 1;
                } else {
                    break;
                }
            }

            messages.insert(key.to_string(), value);
            continue;
        }

        eprintln!(
            "warning: skipping unrecognized line in {}: {}",
            path.display(),
            truncate(trimmed, 60)
        );
        pending_comments.clear();
        i += 1;
    }

    ParsedFtl { messages, contexts }
}

fn find_top_level_eq(line: &str) -> Option<usize> {
    line.find('=')
}

fn read_multiline_value(lines: &[&str], mut i: usize) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with(' ') || line.starts_with('\t') {
            let content = line.trim_start();
            parts.push(content.to_string());
            i += 1;
        } else if line.trim().is_empty() {
            break;
        } else {
            break;
        }
    }
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    (parts.join("\n"), i)
}

fn skip_entry_body(lines: &[&str], mut i: usize) -> usize {
    i += 1;
    while i < lines.len() {
        let t = lines[i];
        if t.starts_with(' ') || t.starts_with('\t') || t.trim().starts_with('.') {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn value_looks_like_selector(v: &str) -> bool {
    v.contains("->") && v.contains('{')
}

fn truncate(s: &str, max: usize) -> String {
    let mut t: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        t.push('…');
    }
    t
}

pub fn render_ftl(messages: &Messages, contexts: &Contexts) -> String {
    let mut out = String::new();
    for (key, value) in messages {
        if let Some(ctx) = contexts.get(key) {
            for line in ctx.lines() {
                if line.is_empty() {
                    out.push_str("#\n");
                } else {
                    out.push_str("# ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        write_entry(&mut out, key, value);
        out.push('\n');
    }
    out
}

fn write_entry(out: &mut String, key: &str, value: &str) {
    let needs_multiline = value.is_empty()
        || value.contains('\n')
        || value.starts_with(' ')
        || value.starts_with('\t')
        || value.starts_with('[')
        || value.starts_with('.');

    if needs_multiline {
        out.push_str(key);
        out.push_str(" =\n");
        if value.is_empty() {
            return;
        }
        for line in value.split('\n') {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    } else {
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(value);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_and_comment() {
        let src = "\
# A greeting
hello = Hello World!
bye = Goodbye
";
        let p = parse_ftl_file(src, Path::new("t.ftl"));
        assert_eq!(p.messages.get("hello").unwrap(), "Hello World!");
        assert_eq!(p.contexts.get("hello").unwrap(), "A greeting");
        assert_eq!(p.messages.get("bye").unwrap(), "Goodbye");
    }

    #[test]
    fn roundtrip_godot_placeholder() {
        let mut msgs = Messages::new();
        msgs.insert(
            "ERROR_DEVICE_NOT_FOUND".into(),
            "No {0} detected.".into(),
        );
        let s = render_ftl(&msgs, &Contexts::new());
        assert!(s.contains("No {0} detected."));
        let p = parse_ftl_file(&s, Path::new("t.ftl"));
        assert_eq!(
            p.messages.get("ERROR_DEVICE_NOT_FOUND").unwrap(),
            "No {0} detected."
        );
    }

    #[test]
    fn multiline_value() {
        let src = "\
blurb =
    line one
    line two
";
        let p = parse_ftl_file(src, Path::new("t.ftl"));
        assert_eq!(p.messages.get("blurb").unwrap(), "line one\nline two");
    }

    #[test]
    fn skip_term_and_attr() {
        let src = "\
-brand = Foo
msg = Hi
    .tooltip = tip
";
        let p = parse_ftl_file(src, Path::new("t.ftl"));
        assert!(p.messages.get("-brand").is_none());
        assert_eq!(p.messages.get("msg").unwrap(), "Hi");
    }

    #[test]
    fn id_validation() {
        assert!(is_valid_ftl_id("GAME_TITLE"));
        assert!(is_valid_ftl_id("hello-world"));
        assert!(!is_valid_ftl_id("1bad"));
        assert!(!is_valid_ftl_id("has space"));
    }
}
