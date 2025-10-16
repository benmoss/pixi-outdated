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

#[derive(Debug, Deserialize, Clone, PartialEq)]
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
