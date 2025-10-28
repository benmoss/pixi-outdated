use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Helper to get the path to the examples directory
fn get_example_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(file)
}

/// Helper to create a command for the binary
fn cmd() -> Command {
    Command::cargo_bin("pixi-outdated").unwrap()
}

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("out-of-date dependencies"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pixi-outdated"));
}

#[test]
fn test_basic_run_with_example_project() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .assert()
        .success();
}

#[test]
fn test_json_output() {
    let manifest_path = get_example_path("pixi.toml");

    let output = cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--json")
        .assert()
        .success();

    // Check that output is valid JSON
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    if !stdout.trim().is_empty() {
        // If there's output, it should be valid JSON
        let _: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");
    }
}

#[test]
fn test_explicit_flag() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--explicit")
        .assert()
        .success();
}

#[test]
fn test_verbose_flag() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--verbose")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Running pixi-outdated with options:",
        ));
}

#[test]
fn test_specific_package() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("python")
        .assert()
        .success();
}

#[test]
fn test_platform_flag() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--platform")
        .arg("linux-64")
        .assert()
        .success();
}

#[test]
fn test_environment_flag() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--environment")
        .arg("default")
        .assert()
        .success();
}

#[test]
fn test_nonexistent_manifest() {
    cmd()
        .arg("--manifest")
        .arg("/nonexistent/pixi.toml")
        .assert()
        .failure();
}

#[test]
fn test_json_and_verbose_together() {
    let manifest_path = get_example_path("pixi.toml");

    let output = cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--json")
        .arg("--verbose")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should contain verbose output
    assert!(stdout.contains("Running pixi-outdated with options:"));

    // Should also be able to extract JSON from the output
    // (it might be mixed with verbose output)
}

#[test]
fn test_multiple_packages() {
    let manifest_path = get_example_path("pixi.toml");

    cmd()
        .arg("--manifest")
        .arg(manifest_path)
        .arg("python")
        .arg("icu")
        .assert()
        .success();
}
