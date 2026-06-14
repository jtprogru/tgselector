//! tgselector — find Telegram-export posts by id and print their processed text.

mod model;
mod render;

use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::Shell;
use serde::Serialize;

use model::{Export, Message};
use render::Format;

/// Find posts by id in a Telegram Desktop JSON export and print their text.
#[derive(Parser, Debug)]
#[command(name = "tgselector", version, about)]
struct Cli {
    /// Path to the Telegram export JSON (result.json).
    #[arg(short, long, value_name = "PATH")]
    file: Option<PathBuf>,

    /// Message id(s) to extract, comma-separated (e.g. `4,5,19`).
    #[arg(short, long, value_name = "IDS", value_delimiter = ',')]
    id: Vec<i64>,

    /// List recent messages as `id <tab> date <tab> preview` (last 10 by default).
    #[arg(short, long)]
    list: bool,

    /// With --list, show every message instead of only the most recent ones.
    #[arg(long)]
    list_full: bool,

    /// With --list, how many recent messages to show (ignored by --list-full).
    #[arg(long, value_name = "N", default_value_t = 10)]
    list_limit: usize,

    /// Preview width (chars) used in --list mode.
    #[arg(long, value_name = "N", default_value_t = 60)]
    list_width: usize,

    /// Output format for extracted posts.
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,

    /// Append a metadata footer (date, source link, author, repost, media).
    #[arg(long)]
    meta: bool,

    /// Emit a JSON structure instead of text (for downstream processing).
    #[arg(long)]
    json: bool,

    /// Public channel @username, used to build `t.me/<channel>/<id>` links.
    /// Without it, links fall back to the `t.me/c/<id>/<msg>` form.
    #[arg(long, value_name = "USERNAME")]
    channel: Option<String>,

    /// Print a shell completion script for the given shell and exit.
    #[arg(long, value_enum, value_name = "SHELL")]
    completions: Option<Shell>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Markdown,
    Plain,
}

impl OutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Markdown => "markdown",
            OutputFormat::Plain => "plain",
        }
    }
}

impl From<OutputFormat> for Format {
    fn from(f: OutputFormat) -> Self {
        match f {
            OutputFormat::Markdown => Format::Markdown,
            OutputFormat::Plain => Format::Plain,
        }
    }
}

/// A post serialized for `--json` output.
#[derive(Serialize)]
struct PostOut {
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    forwarded_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<String>,
    format: &'static str,
    text: String,
}

/// A list entry serialized for `--list --json` output.
#[derive(Serialize)]
struct ListItemOut {
    id: i64,
    date: String,
    preview: String,
}

/// Exit code returned when one or more requested ids are not present.
const EXIT_NOT_FOUND: u8 = 2;

/// Divider printed between multiple extracted posts.
const POST_SEPARATOR: &str = "\n\n═══════════════\n\n";

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Completion scripts need no input file; handle and exit early.
    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "tgselector", &mut io::stdout());
        return ExitCode::SUCCESS;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let code = match run(&cli, &mut out).and_then(|c| {
        out.flush()?;
        Ok(c)
    }) {
        Ok(code) => code,
        // A closed pipe (e.g. `tgselector --list | head`) is a clean exit.
        Err(err) if is_broken_pipe(&err) => return ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::FAILURE;
        }
    };
    ExitCode::from(code)
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
}

/// Builds a post link, preferring a public `t.me/<channel>/<id>` URL when a
/// channel username is supplied, falling back to the `t.me/c/<id>/<msg>` form.
fn post_link(chat_id: Option<i64>, channel: Option<&str>, msg_id: i64) -> Option<String> {
    if let Some(ch) = channel
        .map(|c| c.trim_start_matches('@'))
        .filter(|c| !c.is_empty())
    {
        return Some(format!("https://t.me/{ch}/{msg_id}"));
    }
    chat_id.map(|id| format!("https://t.me/c/{id}/{msg_id}"))
}

/// Loads the export and either lists messages or extracts the requested ids.
/// Returns the process exit code.
fn run<W: Write>(cli: &Cli, out: &mut W) -> Result<u8> {
    let Some(file) = cli.file.as_ref() else {
        bail!("--file <PATH> is required");
    };

    let data = std::fs::read_to_string(file)
        .with_context(|| format!("cannot read file {}", file.display()))?;
    let export: Export = serde_json::from_str(&data)
        .with_context(|| format!("cannot parse JSON in {}", file.display()))?;

    if cli.list {
        let limit = if cli.list_full {
            None
        } else {
            Some(cli.list_limit)
        };
        return print_list(&export, limit, cli.list_width, cli.json, out).map(|()| 0);
    }

    if cli.id.is_empty() {
        bail!("specify --id <N[,N,...]> to extract posts, or --list to browse");
    }

    let format = cli.format.into();
    let mut json_items = Vec::new();
    let mut rendered = Vec::new();
    let mut missing = Vec::new();

    for &id in &cli.id {
        let Some(m) = export.messages.iter().find(|m| m.id == id) else {
            missing.push(id);
            continue;
        };
        let link = post_link(export.id, cli.channel.as_deref(), m.id);
        if cli.json {
            json_items.push(PostOut {
                id: m.id,
                date: m.date.clone(),
                link,
                from: m.from.clone(),
                forwarded_from: m.forwarded_from.clone(),
                media: m.media_label(),
                format: cli.format.as_str(),
                text: render::render(&m.entities(), format),
            });
        } else {
            rendered.push(render::render_post(m, link.as_deref(), format, cli.meta));
        }
    }

    if cli.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&json_items)?)?;
    } else if !rendered.is_empty() {
        writeln!(out, "{}", rendered.join(POST_SEPARATOR))?;
    }

    for id in &missing {
        eprintln!(
            "error: message with id {id} not found in {}",
            file.display()
        );
    }

    Ok(if missing.is_empty() {
        0
    } else {
        EXIT_NOT_FOUND
    })
}

/// Prints content messages (limited to the most recent `limit`, or all).
fn print_list<W: Write>(
    export: &Export,
    limit: Option<usize>,
    width: usize,
    json: bool,
    out: &mut W,
) -> Result<()> {
    let msgs: Vec<&Message> = export.messages.iter().filter(|m| m.is_message()).collect();
    let start = match limit {
        Some(n) if msgs.len() > n => msgs.len() - n,
        _ => 0,
    };
    let selected = &msgs[start..];

    if json {
        let items: Vec<ListItemOut> = selected
            .iter()
            .map(|m| ListItemOut {
                id: m.id,
                date: m.date_only().to_string(),
                preview: list_preview(m, width),
            })
            .collect();
        writeln!(out, "{}", serde_json::to_string_pretty(&items)?)?;
        return Ok(());
    }

    for m in selected {
        writeln!(
            out,
            "{}\t{}\t{}",
            m.id,
            m.date_only(),
            list_preview(m, width)
        )?;
    }
    Ok(())
}

/// Single-line preview for a message, falling back to a media marker.
fn list_preview(m: &Message, width: usize) -> String {
    let preview = render::preview(&m.entities(), width);
    if preview.is_empty() {
        m.media_label()
            .map(|l| format!("[{l}]"))
            .unwrap_or_default()
    } else {
        preview
    }
}
