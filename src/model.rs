//! Serde models for a Telegram Desktop JSON export.
//!
//! Only the fields needed to locate a message by `id`, render its text, and
//! build a metadata footer are modelled; all other fields are ignored.

use serde::Deserialize;

/// Top-level export object: `{ "id": ..., "name": ..., "messages": [...] }`.
#[derive(Debug, Deserialize)]
pub struct Export {
    /// Internal chat id, used to build `t.me/c/<id>/<msg>` links.
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

/// A single message (or service event) in the export.
#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: i64,
    /// `"message"` or `"service"`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// ISO 8601 publication date, e.g. `"2025-12-01T18:15:41"`.
    #[serde(default)]
    pub date: Option<String>,
    /// Author / channel name.
    #[serde(default)]
    pub from: Option<String>,
    /// Original source when the post is a repost.
    #[serde(default)]
    pub forwarded_from: Option<String>,
    /// Media kind (`video_file`, `voice_message`, ...) when present.
    #[serde(default)]
    pub media_type: Option<String>,
    /// Path to an attached photo, when present.
    #[serde(default)]
    pub photo: Option<String>,
    /// Path to an attached file, when present.
    #[serde(default)]
    pub file: Option<String>,
    /// Raw text field: either a plain string or a mixed array of fragments.
    /// Used as a fallback when `text_entities` is empty.
    #[serde(default)]
    pub text: TextField,
    /// Normalized parallel array where every fragment is typed
    /// (plain text is wrapped as `{"type":"plain",...}`).
    #[serde(default)]
    pub text_entities: Vec<Entity>,
}

/// A typed text fragment, e.g. `{"type":"bold","text":"..."}` or
/// `{"type":"text_link","text":"...","href":"..."}`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Entity {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
    /// Target URL for `text_link` entities.
    #[serde(default)]
    pub href: Option<String>,
    /// Language hint for `pre` (code block) entities.
    #[serde(default)]
    pub language: Option<String>,
}

/// The `text` field is either a plain string or an array of mixed
/// strings and entity objects. Modelled untagged so serde picks the variant.
#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
pub enum TextField {
    #[default]
    Empty,
    Plain(String),
    Parts(Vec<TextPart>),
}

/// An element of a `text` array: a bare string (implicitly plain) or an entity.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TextPart {
    Plain(String),
    Entity(Entity),
}

impl Message {
    /// True for regular content messages (not service events).
    pub fn is_message(&self) -> bool {
        self.kind == "message"
    }

    /// Returns the message's content as a uniform list of typed entities.
    ///
    /// Prefers `text_entities` (always typed). Falls back to the raw `text`
    /// field when `text_entities` is empty, normalizing bare strings to
    /// `plain` entities.
    pub fn entities(&self) -> Vec<Entity> {
        if !self.text_entities.is_empty() {
            return self.text_entities.clone();
        }
        match &self.text {
            TextField::Empty => Vec::new(),
            TextField::Plain(s) => vec![plain(s)],
            TextField::Parts(parts) => parts
                .iter()
                .map(|p| match p {
                    TextPart::Plain(s) => plain(s),
                    TextPart::Entity(e) => e.clone(),
                })
                .collect(),
        }
    }

    /// Date portion of the publication timestamp (`YYYY-MM-DD`), or "".
    pub fn date_only(&self) -> &str {
        match self.date.as_deref() {
            Some(d) if d.len() >= 10 => &d[..10],
            Some(d) => d,
            None => "",
        }
    }

    /// Pretty `YYYY-MM-DD HH:MM` timestamp, when a date is present.
    pub fn datetime_pretty(&self) -> Option<String> {
        let d = self.date.as_deref()?;
        if d.len() >= 16 && d.as_bytes()[10] == b'T' {
            Some(format!("{} {}", &d[..10], &d[11..16]))
        } else {
            Some(d.to_string())
        }
    }

    /// Human-readable media label (`photo`, media type, or `file: ...`).
    pub fn media_label(&self) -> Option<String> {
        if let Some(mt) = self.media_type.as_deref().filter(|s| !s.is_empty()) {
            return Some(mt.to_string());
        }
        if self.photo.is_some() {
            return Some("photo".to_string());
        }
        if let Some(f) = self.file.as_deref().filter(|s| !s.is_empty()) {
            return Some(format!("file: {f}"));
        }
        None
    }
}

fn plain(text: &str) -> Entity {
    Entity {
        kind: "plain".to_string(),
        text: text.to_string(),
        href: None,
        language: None,
    }
}
