//! Rendering of a message's entities into a chosen output format, plus a
//! metadata footer and one-line previews.

use crate::model::{Entity, Message};

/// Output format for the processed post text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Plain,
}

/// Renders entities into the given format.
pub fn render(entities: &[Entity], format: Format) -> String {
    match format {
        Format::Markdown => render_markdown(entities),
        Format::Plain => render_plain(entities),
    }
}

/// Renders a full post: body text plus an optional metadata footer.
pub fn render_post(
    message: &Message,
    link: Option<&str>,
    format: Format,
    with_meta: bool,
) -> String {
    let body = render(&message.entities(), format);
    if !with_meta {
        return body;
    }
    let footer = meta_footer(message, link);
    if footer.is_empty() {
        body
    } else if body.is_empty() {
        format!("---\n{footer}")
    } else {
        format!("{body}\n\n---\n{footer}")
    }
}

/// Builds the metadata footer (date, source link, author, repost, media).
/// Returns an empty string when no metadata is available.
pub fn meta_footer(m: &Message, link: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(dt) = m.datetime_pretty() {
        lines.push(format!("📅 {dt}"));
    }
    if let Some(link) = link {
        lines.push(format!("🔗 {link}"));
    }
    if let Some(from) = m.from.as_deref().filter(|s| !s.is_empty()) {
        lines.push(format!("👤 {from}"));
    }
    if let Some(fwd) = m.forwarded_from.as_deref().filter(|s| !s.is_empty()) {
        lines.push(format!("♻️ переслано из: {fwd}"));
    }
    if let Some(media) = m.media_label() {
        lines.push(format!("📎 {media}"));
    }
    lines.join("\n")
}

/// Single-line preview: first sentence or `width`-char truncation of the
/// plain text, with whitespace collapsed.
pub fn preview(entities: &[Entity], width: usize) -> String {
    let collapsed: String = render_plain(entities)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let chars: Vec<char> = collapsed.chars().collect();

    if let Some(end) = chars.iter().position(|c| matches!(c, '.' | '!' | '?')) {
        if end < width {
            return chars[..=end].iter().collect();
        }
    }
    if chars.len() <= width {
        return collapsed;
    }
    let head: String = chars[..width].iter().collect();
    format!("{}…", head.trim_end())
}

/// Renders entities as Markdown, preserving Telegram formatting.
pub fn render_markdown(entities: &[Entity]) -> String {
    let mut out = String::new();
    for e in entities {
        match e.kind.as_str() {
            "bold" => push_wrapped(&mut out, "**", &e.text, "**"),
            "italic" => push_wrapped(&mut out, "*", &e.text, "*"),
            "underline" => push_wrapped(&mut out, "<u>", &e.text, "</u>"),
            "strikethrough" => push_wrapped(&mut out, "~~", &e.text, "~~"),
            "code" => push_wrapped(&mut out, "`", &e.text, "`"),
            "spoiler" => push_wrapped(&mut out, "||", &e.text, "||"),
            "pre" => out.push_str(&render_pre(e)),
            "blockquote" => out.push_str(&render_blockquote(&e.text)),
            "text_link" => out.push_str(&render_text_link(e)),
            // plain, link, mention, hashtag, phone, bot_command, custom_emoji
            // and any unknown type: emit the raw text.
            _ => out.push_str(&e.text),
        }
    }
    out
}

/// Renders entities as plain text, stripping all markup.
pub fn render_plain(entities: &[Entity]) -> String {
    let mut out = String::new();
    for e in entities {
        match e.kind.as_str() {
            "text_link" => {
                out.push_str(&e.text);
                if let Some(href) = e.href.as_deref().filter(|h| !h.is_empty()) {
                    out.push_str(" (");
                    out.push_str(href);
                    out.push(')');
                }
            }
            _ => out.push_str(&e.text),
        }
    }
    out
}

/// Wraps `text` in markers, keeping leading/trailing whitespace *outside* the
/// markers so e.g. `bold` with a trailing newline renders as `**text**\n`,
/// not `**text\n**` (which breaks Markdown emphasis).
fn push_wrapped(out: &mut String, open: &str, text: &str, close: &str) {
    let core = text.trim_matches(char::is_whitespace);
    if core.is_empty() {
        out.push_str(text);
        return;
    }
    let lead = &text[..text.len() - text.trim_start().len()];
    let trail = &text[text.trim_end().len()..];
    out.push_str(lead);
    out.push_str(open);
    out.push_str(core);
    out.push_str(close);
    out.push_str(trail);
}

fn render_pre(e: &Entity) -> String {
    let lang = e.language.as_deref().unwrap_or("");
    let body = e.text.trim_matches('\n');
    format!("```{lang}\n{body}\n```")
}

fn render_blockquote(text: &str) -> String {
    text.split('\n')
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_text_link(e: &Entity) -> String {
    match e.href.as_deref().filter(|h| !h.is_empty()) {
        Some(href) => format!("[{}]({})", e.text, href),
        None => e.text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(kind: &str, text: &str) -> Entity {
        Entity {
            kind: kind.to_string(),
            text: text.to_string(),
            href: None,
            language: None,
        }
    }

    #[test]
    fn plain_passthrough() {
        let e = vec![ent("plain", "hello world")];
        assert_eq!(render_markdown(&e), "hello world");
        assert_eq!(render_plain(&e), "hello world");
    }

    #[test]
    fn bold_italic_strike_code() {
        assert_eq!(render_markdown(&[ent("bold", "b")]), "**b**");
        assert_eq!(render_markdown(&[ent("italic", "i")]), "*i*");
        assert_eq!(render_markdown(&[ent("strikethrough", "s")]), "~~s~~");
        assert_eq!(render_markdown(&[ent("code", "lsof")]), "`lsof`");
        assert_eq!(render_plain(&[ent("bold", "b")]), "b");
        assert_eq!(render_plain(&[ent("code", "lsof")]), "lsof");
    }

    #[test]
    fn wrapped_whitespace_moves_outside_markers() {
        // trailing newline must land after the closing marker
        assert_eq!(
            render_markdown(&[ent("bold", "Заголовок\n")]),
            "**Заголовок**\n"
        );
        // leading and trailing space too
        assert_eq!(render_markdown(&[ent("bold", "  hi  ")]), "  **hi**  ");
        // whitespace-only stays as-is (no empty markers)
        assert_eq!(render_markdown(&[ent("bold", "\n")]), "\n");
    }

    #[test]
    fn underline_spoiler() {
        assert_eq!(render_markdown(&[ent("underline", "u")]), "<u>u</u>");
        assert_eq!(render_markdown(&[ent("spoiler", "x")]), "||x||");
        assert_eq!(render_plain(&[ent("spoiler", "x")]), "x");
    }

    #[test]
    fn pre_with_language() {
        let e = Entity {
            kind: "pre".to_string(),
            text: "\necho hi\n".to_string(),
            href: None,
            language: Some("bash".to_string()),
        };
        assert_eq!(render_markdown(&[e]), "```bash\necho hi\n```");
    }

    #[test]
    fn pre_without_language() {
        let e = Entity {
            kind: "pre".to_string(),
            text: "code".to_string(),
            href: None,
            language: None,
        };
        assert_eq!(render_markdown(&[e]), "```\ncode\n```");
    }

    #[test]
    fn blockquote_prefixes_each_line() {
        let e = ent("blockquote", "line one\nline two");
        assert_eq!(render_markdown(&[e]), "> line one\n> line two");
    }

    #[test]
    fn text_link_markdown_and_plain() {
        let e = Entity {
            kind: "text_link".to_string(),
            text: "Репыч на Гитхаб".to_string(),
            href: Some("https://github.com/zxh326/kite".to_string()),
            language: None,
        };
        assert_eq!(
            render_markdown(std::slice::from_ref(&e)),
            "[Репыч на Гитхаб](https://github.com/zxh326/kite)"
        );
        assert_eq!(
            render_plain(&[e]),
            "Репыч на Гитхаб (https://github.com/zxh326/kite)"
        );
    }

    #[test]
    fn link_mention_hashtag_phone_passthrough() {
        assert_eq!(
            render_markdown(&[ent("link", "https://x.io")]),
            "https://x.io"
        );
        assert_eq!(render_markdown(&[ent("mention", "@user")]), "@user");
        assert_eq!(render_markdown(&[ent("hashtag", "#rust")]), "#rust");
        assert_eq!(render_markdown(&[ent("phone", "1234567890")]), "1234567890");
        assert_eq!(
            render_markdown(&[ent("bot_command", "/version")]),
            "/version"
        );
    }

    #[test]
    fn unknown_type_falls_back_to_text() {
        assert_eq!(render_markdown(&[ent("weird_new_type", "raw")]), "raw");
        assert_eq!(render_plain(&[ent("weird_new_type", "raw")]), "raw");
    }

    #[test]
    fn concatenates_mixed_entities() {
        let e = vec![
            ent("plain", "use "),
            ent("code", "lsof"),
            ent("plain", " or "),
            ent("bold", "fuser"),
        ];
        assert_eq!(render_markdown(&e), "use `lsof` or **fuser**");
        assert_eq!(render_plain(&e), "use lsof or fuser");
    }

    #[test]
    fn preview_truncates_long_text() {
        let e = vec![ent("plain", "слово ".repeat(40).as_str())];
        let p = preview(&e, 20);
        assert!(p.chars().count() <= 21, "len was {}", p.chars().count());
        assert!(p.ends_with('…'));
    }

    #[test]
    fn preview_uses_first_sentence() {
        let e = vec![ent("plain", "Короткое. Второе предложение тут.")];
        assert_eq!(preview(&e, 60), "Короткое.");
    }

    #[test]
    fn preview_collapses_whitespace() {
        let e = vec![ent("plain", "  a\n\n  b   c  ")];
        assert_eq!(preview(&e, 60), "a b c");
    }
}
