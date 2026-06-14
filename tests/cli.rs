//! Integration tests: run the built binary against a fixture export.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tgselector"))
}

fn fixture() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.json").to_string()
}

fn run(args: &[&str]) -> (String, String, i32) {
    let out = bin().args(args).output().expect("failed to run binary");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn renders_markdown_by_default() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "5"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "**Linux-совет**\nиспользуй `lsof`:\n```bash\nfor pid in /proc/[0-9]*; do\n    echo \"$pid\"\ndone\n```\nрепа: [Репыч на Гитхаб](https://github.com/zxh326/kite)\n"
    );
}

#[test]
fn renders_plain_with_expanded_link() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "5", "--format", "plain"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "Linux-совет\nиспользуй lsof:\n\nfor pid in /proc/[0-9]*; do\n    echo \"$pid\"\ndone\n\nрепа: Репыч на Гитхаб (https://github.com/zxh326/kite)\n"
    );
}

#[test]
fn renders_array_text_with_link() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "4"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "Полный список: https://example.com/list\n");
}

#[test]
fn empty_text_prints_blank_line() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "7"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "\n");
}

#[test]
fn multiple_ids_separated() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "4,5"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Полный список:"));
    assert!(stdout.contains("**Linux-совет**"));
    assert!(
        stdout.contains("═══"),
        "posts should be separated by a divider"
    );
}

#[test]
fn meta_footer_added_with_flag() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "4", "--meta"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\n---\n"));
    assert!(stdout.contains("📅 2025-12-01 18:15"));
    assert!(stdout.contains("🔗 https://t.me/c/3373671933/4"));
    assert!(stdout.contains("👤 Идеи для постов"));
    assert!(stdout.contains("♻️ переслано из: Kali Linux"));
}

#[test]
fn media_label_in_meta() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "7", "--meta"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("📎 photo"));
}

#[test]
fn no_meta_by_default() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "4"]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("🔗"));
    assert!(!stdout.contains("📅"));
}

#[test]
fn list_mode_lists_content_messages() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--list"]);
    assert_eq!(code, 0);
    // service message id 1 must be skipped
    assert!(!stdout.lines().any(|l| l.starts_with("1\t")));
    // content messages present with id, date, preview
    assert!(stdout.contains("4\t2025-12-01\t"));
    assert!(stdout.contains("5\t2025-12-01\tLinux-совет"));
    // empty-text photo post shows a media marker
    assert!(stdout.contains("7\t2025-12-01\t[photo]"));
}

#[test]
fn partial_not_found_exits_2_but_prints_found() {
    let (stdout, stderr, code) = run(&["--file", &fixture(), "--id", "4,999999"]);
    assert_eq!(code, 2);
    assert!(stdout.contains("Полный список:"));
    assert!(stderr.contains("999999 not found"));
}

#[test]
fn id_not_found_exits_2() {
    let (stdout, stderr, code) = run(&["--file", &fixture(), "--id", "999999"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("not found"));
}

#[test]
fn missing_file_exits_1() {
    let (_stdout, stderr, code) = run(&["--file", "/no/such/file.json", "--id", "1"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("cannot read file"));
}

#[test]
fn no_id_and_no_list_errors() {
    let (_stdout, stderr, code) = run(&["--file", &fixture()]);
    assert_eq!(code, 1);
    assert!(stderr.contains("--id"));
}

#[test]
fn json_output_is_an_array_of_posts() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--id", "4,5", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], 4);
    assert_eq!(arr[0]["format"], "markdown");
    assert_eq!(arr[0]["link"], "https://t.me/c/3373671933/4");
    assert_eq!(arr[0]["forwarded_from"], "Kali Linux");
    assert!(arr[1]["text"].as_str().unwrap().contains("**Linux-совет**"));
}

#[test]
fn json_respects_plain_format() {
    let (stdout, _stderr, code) = run(&[
        "--file",
        &fixture(),
        "--id",
        "5",
        "--json",
        "--format",
        "plain",
    ]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v[0]["format"], "plain");
    assert!(!v[0]["text"].as_str().unwrap().contains("**"));
}

#[test]
fn channel_flag_builds_public_link() {
    let (stdout, _stderr, code) = run(&[
        "--file",
        &fixture(),
        "--id",
        "4",
        "--meta",
        "--channel",
        "@mychan",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("🔗 https://t.me/mychan/4"));
    assert!(!stdout.contains("t.me/c/"));
}

#[test]
fn list_limit_shows_only_most_recent() {
    // content messages in fixture: 4, 5, 7 (chronological); limit 1 -> last one
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--list", "--list-limit", "1"]);
    assert_eq!(code, 0);
    assert!(stdout.lines().any(|l| l.starts_with("7\t")));
    assert!(!stdout.lines().any(|l| l.starts_with("4\t")));
    assert!(!stdout.lines().any(|l| l.starts_with("5\t")));
}

#[test]
fn list_full_shows_all_content_messages() {
    let (stdout, _stderr, code) = run(&["--file", &fixture(), "--list", "--list-full"]);
    assert_eq!(code, 0);
    for id in ["4\t", "5\t", "7\t"] {
        assert!(stdout.lines().any(|l| l.starts_with(id)), "missing {id}");
    }
    assert!(
        !stdout.lines().any(|l| l.starts_with("1\t")),
        "service skipped"
    );
}

#[test]
fn list_json_output() {
    let (stdout, _stderr, code) = run(&[
        "--file",
        &fixture(),
        "--list",
        "--json",
        "--list-limit",
        "1",
    ]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], 7);
    assert_eq!(arr[0]["date"], "2025-12-01");
}

#[test]
fn completions_script_for_each_shell() {
    for shell in ["bash", "zsh", "fish"] {
        let (stdout, _stderr, code) = run(&["--completions", shell]);
        assert_eq!(code, 0, "shell {shell}");
        assert!(
            stdout.contains("tgselector"),
            "shell {shell} script mentions binary"
        );
    }
}
