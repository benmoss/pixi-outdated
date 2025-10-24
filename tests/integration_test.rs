use std::fs;
use tempfile::TempDir;

/// Test with the actual example project in the repository
#[test]
fn test_with_real_example_project() {
    // This assumes we're running from the project root
    let result = pixi_outdated::get_platforms_from_lockfile(Some("examples/pixi.toml"), None);

    // Should succeed
    assert!(
        result.is_ok(),
        "Failed to read example project: {:?}",
        result.err()
    );

    let platforms = result.unwrap();
    assert!(
        !platforms.is_empty(),
        "Example project should have platforms"
    );

    // Example project should have linux-64 and osx-arm64
    assert!(
        platforms.len() >= 2,
        "Example should have multiple platforms"
    );
    assert!(
        platforms.contains(&"linux-64".to_string()),
        "Example should have linux-64"
    );
    assert!(
        platforms.contains(&"osx-arm64".to_string()),
        "Example should have osx-arm64"
    );
}

#[test]
fn test_error_nonexistent_lockfile() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("pixi.toml");
    fs::write(&manifest_path, "").unwrap();

    // No lockfile exists
    let result =
        pixi_outdated::get_platforms_from_lockfile(Some(manifest_path.to_str().unwrap()), None);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to read lockfile"));
}

#[test]
fn test_error_invalid_lockfile_format() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("pixi.toml");
    fs::write(&manifest_path, "").unwrap();

    // Write invalid YAML
    let lockfile_path = temp_dir.path().join("pixi.lock");
    fs::write(&lockfile_path, "this is not valid yaml: {[}").unwrap();

    let result =
        pixi_outdated::get_platforms_from_lockfile(Some(manifest_path.to_str().unwrap()), None);

    assert!(result.is_err());
}

#[test]
fn test_error_missing_environment() {
    // Use the example project but request a non-existent environment
    let result = pixi_outdated::get_platforms_from_lockfile(
        Some("examples/pixi.toml"),
        Some("does-not-exist"),
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Environment 'does-not-exist' not found"));
}

#[test]
fn test_lockfile_without_manifest_path() {
    // Create a test project in a temp dir
    let temp_dir = TempDir::new().unwrap();

    // Copy the example lockfile to the temp directory
    let example_lock = fs::read_to_string("examples/pixi.lock").unwrap();
    fs::write(temp_dir.path().join("pixi.lock"), example_lock).unwrap();

    // Change to the project directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Should work without explicit manifest path (uses current directory)
    let result = pixi_outdated::get_platforms_from_lockfile(None, None);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    assert!(
        result.is_ok(),
        "Should find lockfile in current directory: {:?}",
        result.err()
    );
    let platforms = result.unwrap();
    assert!(platforms.len() >= 2);
}

#[test]
fn test_platforms_are_valid_platform_strings() {
    let result = pixi_outdated::get_platforms_from_lockfile(Some("examples/pixi.toml"), None);
    assert!(result.is_ok());

    let platforms = result.unwrap();

    // All platforms should be valid platform identifiers
    let valid_platforms = [
        "linux-64",
        "osx-64",
        "osx-arm64",
        "win-64",
        "linux-aarch64",
        "linux-ppc64le",
    ];

    for platform in &platforms {
        assert!(
            valid_platforms.contains(&platform.as_str()),
            "Invalid platform: {}",
            platform
        );
    }
}
