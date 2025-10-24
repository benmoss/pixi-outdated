use anyhow::Result;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, serde::Deserialize)]
struct PackageUpdate {
    name: String,
    installed_version: String,
    latest_version: String,
}

#[derive(Parser, Debug)]
#[command(
    name = "pixi-outdated",
    version,
    about = "Check for outdated dependencies in pixi projects",
    long_about = "A CLI tool to determine out-of-date dependencies in pixi.toml/pyproject.toml and pixi.lock files"
)]
struct Cli {
    /// Specific package names to check (if not provided, checks all packages)
    packages: Vec<String>,

    /// Only check packages explicitly listed in pixi.toml (not transitive dependencies)
    #[arg(short = 'x', long)]
    explicit: bool,

    /// The environment to check (defaults to the default environment)
    #[arg(short = 'e', long)]
    environment: Option<String>,

    /// The platform to check (if not specified, checks all common platforms)
    #[arg(short = 'p', long)]
    platform: Option<String>,

    /// Output in JSON format
    #[arg(short, long)]
    json: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Path to the pixi.toml file (defaults to current directory)
    #[arg(short = 'f', long)]
    manifest: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing if verbose mode is enabled
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    "rattler_repodata_gateway=debug,pixi_outdated=debug".into()
                }),
            )
            .init();
    }

    if cli.verbose {
        println!("Running pixi-outdated with options:");
        if let Some(ref manifest) = cli.manifest {
            println!("  Manifest: {}", manifest);
        }
        println!("  Explicit only: {}", cli.explicit);
        if let Some(ref env) = cli.environment {
            println!("  Environment: {}", env);
        }
        if let Some(ref platform) = cli.platform {
            println!("  Platform: {}", platform);
        }
        println!("  JSON output: {}", cli.json);
        if !cli.packages.is_empty() {
            println!("  Checking packages: {}", cli.packages.join(", "));
        } else {
            println!("  Checking all packages");
        }
        println!();
    }

    run(cli).await
}

// Use the library function for getting platforms from lockfile
use pixi_outdated::get_platforms_from_lockfile;

async fn run(cli: Cli) -> Result<()> {
    // Step 1: Get package list from `pixi list --json`
    // Determine which platforms to check
    let platforms_to_check: Vec<String> = if let Some(ref plat) = cli.platform {
        vec![plat.clone()]
    } else {
        // When no platform is specified, read platforms from lockfile for the specified environment
        get_platforms_from_lockfile(cli.manifest.as_deref(), cli.environment.as_deref())?
    };

    let check_multiple_platforms = cli.platform.is_none();

    if cli.verbose && !cli.json && check_multiple_platforms {
        println!("Checking platforms: {}\n", platforms_to_check.join(", "));
    }

    // Track updates per platform (used for both JSON and text output)
    let mut platform_updates: std::collections::HashMap<String, Vec<PackageUpdate>> =
        std::collections::HashMap::new();

    // Collect all packages from all platforms first
    let mut platform_packages: std::collections::HashMap<
        String,
        Vec<pixi_outdated::pixi::PixiPackage>,
    > = std::collections::HashMap::new();

    for platform in &platforms_to_check {
        if cli.verbose && !cli.json {
            println!("Fetching package list for {}...", platform);
        }

        let packages = match pixi_outdated::pixi::get_package_list(
            cli.explicit,
            cli.environment.as_deref(),
            Some(platform.as_str()),
            cli.manifest.as_deref(),
            &cli.packages,
        ) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                // Platform might not be supported, skip it
                if cli.verbose && !cli.json {
                    eprintln!("Skipping platform {}: {}", platform, e);
                }
                continue;
            }
        };

        if packages.is_empty() {
            if cli.verbose && !cli.json {
                println!("No packages found for platform {}", platform);
            }
            continue;
        }

        if cli.verbose && !cli.json {
            println!("Found {} packages\n", packages.len());
        }

        platform_packages.insert(platform.clone(), packages);
    }

    if platform_packages.is_empty() {
        if !cli.json {
            println!("No packages found for any platform");
        }
        return Ok(());
    }

    // Build a unique set of packages to check (package name + channel)
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct PackageKey {
        name: String,
        channel: Option<String>,
        kind: pixi_outdated::pixi::PackageKind,
    }

    let mut unique_packages: std::collections::HashMap<PackageKey, String> =
        std::collections::HashMap::new();

    // Collect unique packages across all platforms
    for packages in platform_packages.values() {
        for package in packages {
            let channel = package
                .source
                .as_ref()
                .and_then(|s| pixi_outdated::conda::extract_channel_url(s));

            let key = PackageKey {
                name: package.name.clone(),
                channel: channel.clone(),
                kind: package.kind,
            };

            // Store the first version we see (they might differ per platform)
            unique_packages
                .entry(key)
                .or_insert(package.version.clone());
        }
    }

    // Create multi-progress for showing progress bars (only if not JSON and not verbose)
    let multi_progress = if !cli.json && !cli.verbose {
        Some(MultiProgress::new())
    } else {
        None
    };

    let progress_bar = if let Some(ref mp) = multi_progress {
        let pb = mp.add(ProgressBar::new(unique_packages.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .expect("Invalid progress bar template")
                .progress_chars("█▓▒░ "),
        );
        pb.set_prefix("Checking".to_string());
        Some(pb)
    } else {
        None
    };

    // Cache for version queries (package_key -> latest_version)
    let mut version_cache: std::collections::HashMap<PackageKey, Option<String>> =
        std::collections::HashMap::new();

    // Query each unique package once
    for (key, _version) in &unique_packages {
        if let Some(ref pb) = progress_bar {
            pb.set_message(key.name.clone());
        }

        match key.kind {
            pixi_outdated::pixi::PackageKind::Conda => {
                if let Some(ref channel_url) = key.channel {
                    if cli.verbose && !cli.json {
                        println!("Checking {} (conda) from {}...", key.name, channel_url);
                    }

                    // Query all platforms at once for efficiency
                    let platform_refs: Vec<&str> =
                        platforms_to_check.iter().map(|s| s.as_str()).collect();
                    let latest_result =
                        pixi_outdated::conda::get_latest_conda_version_multi_platform(
                            &key.name,
                            channel_url,
                            &platform_refs,
                        )
                        .await;

                    match latest_result {
                        Ok(latest) => {
                            version_cache.insert(key.clone(), latest);
                        }
                        Err(e) => {
                            if !cli.json {
                                eprintln!("Error checking {}: {}", key.name, e);
                            }
                            version_cache.insert(key.clone(), None);
                        }
                    }
                } else if cli.verbose && !cli.json {
                    println!(
                        "Skipping {} (conda): unable to extract channel URL",
                        key.name
                    );
                }
            }
            pixi_outdated::pixi::PackageKind::Pypi => {
                if cli.verbose && !cli.json {
                    println!("Checking {} (PyPI)...", key.name);
                }

                match pixi_outdated::pypi::get_latest_pypi_version(&key.name).await {
                    Ok(latest) => {
                        version_cache.insert(key.clone(), Some(latest));
                    }
                    Err(e) => {
                        if !cli.json {
                            eprintln!("Error checking {}: {}", key.name, e);
                        }
                        version_cache.insert(key.clone(), None);
                    }
                }
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(ref pb) = progress_bar {
        pb.finish_with_message("Done");
    }

    // Now build updates per platform using the cached results
    for (platform, packages) in &platform_packages {
        let mut platform_package_updates: Vec<PackageUpdate> = Vec::new();

        for package in packages {
            let channel = package
                .source
                .as_ref()
                .and_then(|s| pixi_outdated::conda::extract_channel_url(s));

            let key = PackageKey {
                name: package.name.clone(),
                channel,
                kind: package.kind,
            };

            if let Some(Some(latest)) = version_cache.get(&key) {
                if latest != &package.version {
                    let update = PackageUpdate {
                        name: package.name.clone(),
                        installed_version: package.version.clone(),
                        latest_version: latest.clone(),
                    };
                    platform_package_updates.push(update);
                } else if cli.verbose && !cli.json {
                    println!("{}: {} (up to date)", package.name, package.version);
                }
            } else if cli.verbose && !cli.json {
                println!(
                    "{}: {} (no newer version found)",
                    package.name, package.version
                );
            }
        }

        platform_updates.insert(platform.clone(), platform_package_updates);
    }

    // Output results
    if cli.json {
        // JSON output: grouped by platform
        println!("{}", serde_json::to_string_pretty(&platform_updates)?);
    } else if check_multiple_platforms {
        // Coalesce updates: find packages that have the same update across ALL platforms
        let mut common_updates: Vec<PackageUpdate> = Vec::new();
        let mut platform_specific_updates: std::collections::HashMap<String, Vec<PackageUpdate>> =
            std::collections::HashMap::new();

        if !platform_updates.is_empty() {
            // Get the first platform's updates as candidates for common updates
            let platforms: Vec<String> = platform_updates.keys().cloned().collect();

            if let Some(first_platform) = platforms.first() {
                if let Some(first_updates) = platform_updates.get(first_platform) {
                    for update in first_updates {
                        // Check if this exact update exists in all other platforms
                        let is_common = platforms.iter().skip(1).all(|plat| {
                            platform_updates.get(plat).is_some_and(|updates| {
                                updates.iter().any(|u| {
                                    u.name == update.name
                                        && u.installed_version == update.installed_version
                                        && u.latest_version == update.latest_version
                                })
                            })
                        });

                        if is_common && platforms.len() > 1 {
                            common_updates.push(update.clone());
                        }
                    }
                }
            }

            // Now collect platform-specific updates (excluding common ones)
            for (platform, updates) in &platform_updates {
                let specific: Vec<PackageUpdate> = updates
                    .iter()
                    .filter(|update| {
                        !common_updates.iter().any(|common| {
                            common.name == update.name
                                && common.installed_version == update.installed_version
                                && common.latest_version == update.latest_version
                        })
                    })
                    .cloned()
                    .collect();

                if !specific.is_empty() {
                    platform_specific_updates.insert(platform.clone(), specific);
                }
            }
        }

        // Print common updates first
        if !common_updates.is_empty() {
            println!("\n=== All Platforms ===");
            for update in &common_updates {
                println!(
                    "{}: {} -> {}",
                    update.name, update.installed_version, update.latest_version
                );
            }
        }

        // Print platform-specific updates
        for platform in &platforms_to_check {
            if let Some(updates) = platform_specific_updates.get(platform) {
                if !updates.is_empty() {
                    println!("\n=== Platform: {} ===", platform);
                    for update in updates {
                        println!(
                            "{}: {} -> {}",
                            update.name, update.installed_version, update.latest_version
                        );
                    }
                }
            }
        }

        println!("\nAnalysis complete!");
    } else {
        // Single platform output
        if let Some(updates) = platform_updates.values().next() {
            for update in updates {
                println!(
                    "{}: {} -> {}",
                    update.name, update.installed_version, update.latest_version
                );
            }
        }
        println!("\nAnalysis complete!");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_key_uniqueness() {
        use std::collections::HashMap;

        #[derive(Hash, Eq, PartialEq, Clone)]
        struct PackageKey {
            name: String,
            channel: Option<String>,
            kind: pixi_outdated::pixi::PackageKind,
        }

        let mut map: HashMap<PackageKey, String> = HashMap::new();

        // Same package, same channel, same kind - should be same key
        let key1 = PackageKey {
            name: "python".to_string(),
            channel: Some("https://conda.anaconda.org/conda-forge/".to_string()),
            kind: pixi_outdated::pixi::PackageKind::Conda,
        };
        let key2 = PackageKey {
            name: "python".to_string(),
            channel: Some("https://conda.anaconda.org/conda-forge/".to_string()),
            kind: pixi_outdated::pixi::PackageKind::Conda,
        };

        map.insert(key1, "3.12.0".to_string());
        map.insert(key2, "3.12.1".to_string());

        // Should only have one entry (keys are equal)
        assert_eq!(map.len(), 1);
        assert_eq!(map.values().next().unwrap(), "3.12.1");

        // Different channel - should be different key
        let key3 = PackageKey {
            name: "python".to_string(),
            channel: Some("https://conda.anaconda.org/main/".to_string()),
            kind: pixi_outdated::pixi::PackageKind::Conda,
        };
        map.insert(key3, "3.11.0".to_string());
        assert_eq!(map.len(), 2);

        // PyPI package (no channel) - should be different from conda
        let key4 = PackageKey {
            name: "python".to_string(),
            channel: None,
            kind: pixi_outdated::pixi::PackageKind::Pypi,
        };
        map.insert(key4, "3.13.0".to_string());
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_package_update_serialization() {
        let update = PackageUpdate {
            name: "python".to_string(),
            installed_version: "3.12.0".to_string(),
            latest_version: "3.13.0".to_string(),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("python"));
        assert!(json.contains("3.12.0"));
        assert!(json.contains("3.13.0"));

        let deserialized: PackageUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, update.name);
        assert_eq!(deserialized.installed_version, update.installed_version);
        assert_eq!(deserialized.latest_version, update.latest_version);
    }
}
