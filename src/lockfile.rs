use anyhow::{Context, Result};
use rattler_lock::LockFile;
use std::path::Path;

/// Read platforms from the pixi.lock file for a specific environment
pub fn get_platforms_from_lockfile(
    manifest_path: Option<&str>,
    environment: Option<&str>,
) -> Result<Vec<String>> {
    // Find the lockfile path
    let lockfile_path = if let Some(manifest) = manifest_path {
        let manifest_dir = Path::new(manifest)
            .parent()
            .context("Failed to get manifest directory")?;
        manifest_dir.join("pixi.lock")
    } else {
        Path::new("pixi.lock").to_path_buf()
    };

    // Read the lockfile
    let lockfile = LockFile::from_path(&lockfile_path)
        .with_context(|| format!("Failed to read lockfile at {}", lockfile_path.display()))?;

    // Find the specified environment (or use default)
    let env_name = environment.unwrap_or("default");

    let (_name, env) = lockfile
        .environments()
        .find(|(name, _env)| *name == env_name)
        .with_context(|| format!("Environment '{}' not found in lockfile", env_name))?;

    // Extract platforms from the environment
    let platforms: Vec<String> = env.platforms().map(|p| p.to_string()).collect();

    if platforms.is_empty() {
        anyhow::bail!("No platforms found for environment '{}'", env_name);
    }

    Ok(platforms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_platforms_from_example_project() {
        // Test with the actual example project
        let result = get_platforms_from_lockfile(Some("examples/pixi.toml"), None);

        assert!(result.is_ok());
        let platforms = result.unwrap();

        assert!(platforms.len() >= 2);
        assert!(platforms.contains(&"linux-64".to_string()));
        assert!(platforms.contains(&"osx-arm64".to_string()));
    }

    #[test]
    fn test_get_platforms_missing_env() {
        // Use the example project but request a non-existent environment
        let result = get_platforms_from_lockfile(Some("examples/pixi.toml"), Some("nonexistent"));

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Environment 'nonexistent' not found"),
            "Expected error message to contain \"Environment 'nonexistent' not found\", but got: {}",
            error_msg
        );
    }

    #[test]
    fn test_get_platforms_missing_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("pixi.toml");
        fs::write(&manifest_path, "").unwrap();

        let result = get_platforms_from_lockfile(Some(manifest_path.to_str().unwrap()), None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read lockfile"));
    }

    #[test]
    fn test_get_platforms_invalid_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("pixi.toml");
        let lockfile_path = temp_dir.path().join("pixi.lock");

        fs::write(&manifest_path, "").unwrap();
        fs::write(&lockfile_path, "invalid yaml {[}").unwrap();

        let result = get_platforms_from_lockfile(Some(manifest_path.to_str().unwrap()), None);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_platforms_without_manifest_path() {
        // Create a temp dir and copy the example lockfile
        let temp_dir = TempDir::new().unwrap();
        let example_lock = fs::read_to_string("examples/pixi.lock").unwrap();
        fs::write(temp_dir.path().join("pixi.lock"), example_lock).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Should find lockfile in current directory
        let result = get_platforms_from_lockfile(None, None);

        // Restore directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let platforms = result.unwrap();
        assert!(platforms.len() >= 2);
    }
}
