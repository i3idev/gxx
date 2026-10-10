//! Integration tests for CLI command dispatch.
//!
//! These tests verify that the `gxx` binary correctly routes commands.
//! GUI-launching commands (`app`, `-r`, `--read`) are tested at the
//! dispatch level — in a headless environment, `app` will fail to create
//! a Slint window, but we verify it does NOT fall through to the
//! vocabulary handler (which would print "not found: app").

use assert_cmd::Command;
use predicates::prelude::*;

/// `gxx app` must NOT be treated as a vocabulary lookup.
///
/// In a headless environment, `gxx app` will fail because there is no
/// display server. The key assertion is that the error is a GUI/platform
/// error, NOT "not found: app (try: gxx find app)" which would indicate
/// the command fell through to `cmd_show`.
#[test]
fn test_app_command_not_treated_as_word_lookup() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("app");
    let output = cmd.timeout(std::time::Duration::from_secs(5)).output();

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stderr}\n{stdout}");
        assert!(
            !combined.contains("not found: app"),
            "gxx app must not be routed to vocabulary lookup. Got: {combined}"
        );
        assert!(
            !combined.contains("try: gxx find"),
            "gxx app must not suggest gxx find. Got: {combined}"
        );
    }
}

/// `gxx -r <nonexistent>` should report a file error, not "not found".
#[test]
fn test_read_mode_missing_file() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("-r").arg("/nonexistent/path/file.txt");
    cmd.assert().failure().stderr(
        predicate::str::contains("Failed to read file")
            .or(predicate::str::contains("No such file")),
    );
}

/// `gxx --read <nonexistent>` should behave the same as `-r`.
#[test]
fn test_read_long_alias_missing_file() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("--read").arg("/nonexistent/path/file2.txt");
    cmd.assert().failure().stderr(
        predicate::str::contains("Failed to read file")
            .or(predicate::str::contains("No such file")),
    );
}

/// Existing word commands still work (e.g., `gxx add` without args gives usage).
#[test]
fn test_existing_commands_still_work() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("add");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("usage").or(predicate::str::contains("word")));
}

/// `gxx help` should still work and show usage.
#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("gxx - bilingual"));
}

/// `gxx --help` should still work and show usage.
#[test]
fn test_help_long_alias() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("gxx - bilingual"));
}

/// Verify the `-r` flag requires a filename.
#[test]
fn test_read_requires_filename() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("-r");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("usage: gxx -r <file.txt>"));
}

/// Verify that `--read` without a filename also shows usage.
#[test]
fn test_read_long_alias_requires_filename() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("--read");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("usage: gxx -r <file.txt>"));
}

/// Verify that `gxx` with no args prints usage (exit code 0).
#[test]
fn test_no_args_prints_usage() {
    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("gxx - bilingual"));
}

/// Direct Reading Mode (`-r`) with a real file should attempt to open the
/// GUI. In a headless environment it may fail at window creation, but it
/// should NOT report "not found" or route to vocabulary lookup.
#[test]
fn test_read_mode_with_real_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("gxx-cli-test-input.txt");
    std::fs::write(&test_file, "Hello world. This is a GXX reading test.").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("-r").arg(&test_file);

    let output = cmd.timeout(std::time::Duration::from_secs(10)).output();

    let _ = std::fs::remove_file(&test_file);

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stderr}\n{stdout}");
        assert!(
            !combined.contains("not found:") && !combined.contains("try: gxx find"),
            "gxx -r should not route to vocabulary lookup. Got: {combined}"
        );
    }
}

/// `gxx init` should work (database initialization).
#[test]
fn test_init_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let home = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.env("GXX_HOME", home);
    cmd.arg("init");
    cmd.assert().success();
}

/// `gxx db list` should work after init.
#[test]
fn test_db_list_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let home = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.env("GXX_HOME", home);
    cmd.arg("init");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.env("GXX_HOME", home);
    cmd.arg("db").arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no databases").or(predicate::str::contains("Databases")));
}
