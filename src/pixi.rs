use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize, Clone)]
pub struct PixiPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    pub kind: PackageKind,
    #[serde(default)]
    pub source: Option<String>,
    pub is_explicit: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    Conda,
    Pypi,
}

/// Get the list of packages from `pixi list --json`
pub fn get_package_list(
    explicit: bool,
    environment: Option<&str>,
    platform: Option<&str>,
    manifest: Option<&str>,
    package_names: &[String],
) -> Result<Vec<PixiPackage>> {
    let mut cmd = Command::new("pixi");
    cmd.arg("list").arg("--json");

    if explicit {
        cmd.arg("--explicit");
    }

    if let Some(env) = environment {
        cmd.arg("--environment").arg(env);
    }

    if let Some(plat) = platform {
        cmd.arg("--platform").arg(plat);
    }

    if let Some(man) = manifest {
        cmd.arg("--manifest-path").arg(man);
    }

    // If package names are specified, create a regex pattern to match them
    // For a single package, just use the name directly
    // For multiple packages, create a pattern like '^(pkg1|pkg2|pkg3)$'
    if !package_names.is_empty() {
        let regex_pattern = if package_names.len() == 1 {
            // For a single package, just use the name (pixi will match it as a regex)
            format!("^{}$", regex::escape(&package_names[0]))
        } else {
            // For multiple packages, create an alternation pattern
            let escaped_names: Vec<String> = package_names
                .iter()
                .map(|name| regex::escape(name))
                .collect();
            format!("^({})$", escaped_names.join("|"))
        };
        cmd.arg(&regex_pattern);
    }

    let output = cmd
        .output()
        .context("Failed to execute `pixi list`. Is pixi installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pixi list failed: {}", stderr);
    }

    let stdout =
        String::from_utf8(output.stdout).context("pixi list output was not valid UTF-8")?;

    let packages: Vec<PixiPackage> = serde_json::from_str(&stdout).with_context(|| {
        format!(
            "Failed to parse JSON output from pixi list. Output was:\n{}",
            stdout
        )
    })?;

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_kind_traits() {
        // Test that PackageKind has the required traits for HashMap keys
        use std::collections::HashMap;

        let mut map: HashMap<PackageKind, String> = HashMap::new();
        map.insert(PackageKind::Conda, "conda_value".to_string());
        map.insert(PackageKind::Pypi, "pypi_value".to_string());

        assert_eq!(
            map.get(&PackageKind::Conda),
            Some(&"conda_value".to_string())
        );
        assert_eq!(map.get(&PackageKind::Pypi), Some(&"pypi_value".to_string()));
        assert_eq!(map.len(), 2);

        // Test Copy trait
        let kind1 = PackageKind::Conda;
        let kind2 = kind1; // This should compile because PackageKind implements Copy
        assert_eq!(kind1, kind2);
    }

    #[test]
    fn test_pixi_package_deserialization() {
        let json = r#"{
            "name": "python",
            "version": "3.12.0",
            "build": "h1234567_0",
            "size_bytes": 12345678,
            "kind": "conda",
            "source": "https://conda.anaconda.org/conda-forge/linux-64/python-3.12.0.tar.bz2",
            "is_explicit": true
        }"#;

        let package: PixiPackage = serde_json::from_str(json).unwrap();

        assert_eq!(package.name, "python");
        assert_eq!(package.version, "3.12.0");
        assert_eq!(package.build, Some("h1234567_0".to_string()));
        assert_eq!(package.size_bytes, Some(12345678));
        assert_eq!(package.kind, PackageKind::Conda);
        assert!(package.source.is_some());
        assert!(package.is_explicit);
    }

    #[test]
    fn test_pixi_package_deserialization_minimal() {
        // Test with minimal fields (optional fields missing)
        let json = r#"{
            "name": "cowsay",
            "version": "5.0",
            "kind": "pypi",
            "is_explicit": false
        }"#;

        let package: PixiPackage = serde_json::from_str(json).unwrap();

        assert_eq!(package.name, "cowsay");
        assert_eq!(package.version, "5.0");
        assert_eq!(package.build, None);
        assert_eq!(package.size_bytes, None);
        assert_eq!(package.kind, PackageKind::Pypi);
        assert_eq!(package.source, None);
        assert!(!package.is_explicit);
    }

    #[test]
    fn test_pixi_package_clone() {
        let package = PixiPackage {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            build: Some("build123".to_string()),
            size_bytes: Some(1000),
            kind: PackageKind::Conda,
            source: Some("https://example.com/package.tar.bz2".to_string()),
            is_explicit: true,
        };

        let cloned = package.clone();

        assert_eq!(cloned.name, package.name);
        assert_eq!(cloned.version, package.version);
        assert_eq!(cloned.build, package.build);
        assert_eq!(cloned.size_bytes, package.size_bytes);
        assert_eq!(cloned.kind, package.kind);
        assert_eq!(cloned.source, package.source);
        assert_eq!(cloned.is_explicit, package.is_explicit);
    }
}
