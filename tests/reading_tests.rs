//! Integration tests for the `read` subcommand.
//!
//! These tests verify that `gxx read <file>` handles files appropriately
//! and routes to TTS rather than falling through to vocabulary lookup or
//! the GUI. Empty and whitespace-only files must be rejected.

use assert_cmd::Command;
use predicates::prelude::*;

/// `read` with an empty file should report an error.
#[test]
fn test_read_mode_empty_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("empty.txt");
    std::fs::write(&test_file, "").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("read").arg(&test_file);
    cmd.assert().failure().stderr(
        predicate::str::contains("no readable text")
            .or(predicate::str::contains("empty"))
            .or(predicate::str::contains("No words")),
    );
}

/// `read` with a whitespace-only file should report an error.
#[test]
fn test_read_mode_whitespace_only_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("blank.txt");
    std::fs::write(&test_file, "   \n\n  \t  ").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("read").arg(&test_file);
    cmd.assert().failure().stderr(
        predicate::str::contains("no readable text")
            .or(predicate::str::contains("empty"))
            .or(predicate::str::contains("No words")),
    );
}

/// `read` as a bare command (not just `-r`) should also trigger Reading Mode.
#[test]
fn test_read_command_long_form() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test_read.txt");
    std::fs::write(&test_file, "Hello world.").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("read").arg(&test_file);

    let output = cmd.timeout(std::time::Duration::from_secs(10)).output();

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stderr}\n{stdout}");
        assert!(
            !combined.contains("not found: read"),
            "gxx read should not route to vocabulary lookup. Got: {combined}"
        );
    }
}

/// Reading Mode input file should not be confused with the `input_text`
/// property used in the main AppWindow.
#[test]
fn test_reading_mode_separate_from_app_window() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("separate_test.txt");
    std::fs::write(&test_file, "Test separation.").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("read").arg(&test_file);

    let output = cmd.timeout(std::time::Duration::from_secs(10)).output();

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("AppWindow"),
            "Should not reference AppWindow in reading mode. Got: {stderr}"
        );
    }
}
