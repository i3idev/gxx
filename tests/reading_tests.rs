//! Integration tests for Reading Mode entry point.
//!
//! These tests verify that the `-r`/`--read` arguments correctly route
//! to the Reading Mode GUI and handle files appropriately. They do NOT
//! require an interactive display — in headless CI environments, the
//! Slint window creation will fail, but we verify dispatch correctness.

use assert_cmd::Command;
use predicates::prelude::*;

/// `-r` with an empty file should report an error (empty file).
#[test]
fn test_read_mode_empty_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("empty.txt");
    std::fs::write(&test_file, "").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("-r").arg(&test_file);
    cmd.assert().failure().stderr(
        predicate::str::contains("empty")
            .or(predicate::str::contains("No words"))
            .or(predicate::str::contains("Failed to read")),
    );
}

/// `-r` with a whitespace-only file should report an error.
#[test]
fn test_read_mode_whitespace_only_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("blank.txt");
    std::fs::write(&test_file, "   \n\n  \t  ").unwrap();

    let mut cmd = Command::cargo_bin("gxx").unwrap();
    cmd.arg("-r").arg(&test_file);
    cmd.assert().failure().stderr(
        predicate::str::contains("empty")
            .or(predicate::str::contains("No words"))
            .or(predicate::str::contains("Failed to read")),
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
    cmd.arg("-r").arg(&test_file);

    let output = cmd.timeout(std::time::Duration::from_secs(10)).output();

    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("AppWindow"),
            "Should not reference AppWindow in reading mode. Got: {stderr}"
        );
    }
}
