use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rattler_lock::LockFile;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
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

/// Read platforms from the pixi.lock file for a specific environment
fn get_platforms_from_lockfile(
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

    // Create multi-progress for showing progress bars (only if not JSON and not verbose)
    let multi_progress = if !cli.json && !cli.verbose {
        Some(MultiProgress::new())
    } else {
        None
    };

    for platform in &platforms_to_check {
        // Fetch package list for this specific platform
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

        // Create progress bar for this platform
        let progress_bar = if let Some(ref mp) = multi_progress {
            let pb = mp.add(ProgressBar::new(packages.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("█▓▒░ "),
            );
            pb.set_prefix(platform.to_string());
            Some(pb)
        } else {
            None
        };

        // Collect updates for this platform
        let mut platform_package_updates: Vec<PackageUpdate> = Vec::new();

        for package in &packages {
            // Update progress bar message
            if let Some(ref pb) = progress_bar {
                pb.set_message(package.name.to_string());
            }

            match package.kind {
                pixi_outdated::pixi::PackageKind::Conda => {
                    // Extract channel URL from the source
                    if let Some(ref source) = package.source {
                        if let Some(channel_url) = pixi_outdated::conda::extract_channel_url(source)
                        {
                            if cli.verbose && !cli.json {
                                println!(
                                    "Checking {} (conda) from {}...",
                                    package.name, channel_url
                                );
                            }

                            // If checking multiple platforms, query all platforms at once for efficiency
                            let latest_result = if check_multiple_platforms {
                                let platform_refs: Vec<&str> =
                                    platforms_to_check.iter().map(|s| s.as_str()).collect();
                                pixi_outdated::conda::get_latest_conda_version_multi_platform(
                                    &package.name,
                                    &channel_url,
                                    &platform_refs,
                                )
                                .await
                            } else {
                                pixi_outdated::conda::get_latest_conda_version(
                                    &package.name,
                                    &channel_url,
                                    platform.as_str(),
                                )
                                .await
                            };

                            match latest_result {
                                Ok(Some(latest)) => {
                                    if latest != package.version {
                                        let update = PackageUpdate {
                                            name: package.name.clone(),
                                            installed_version: package.version.clone(),
                                            latest_version: latest,
                                        };
                                        platform_package_updates.push(update);
                                    } else if cli.verbose && !cli.json {
                                        println!(
                                            "{}: {} (up to date)",
                                            package.name, package.version
                                        );
                                    }
                                }
                                Ok(None) => {
                                    if cli.verbose && !cli.json {
                                        println!(
                                            "{}: {} (no newer version found)",
                                            package.name, package.version
                                        );
                                    }
                                }
                                Err(e) => {
                                    if !cli.json {
                                        eprintln!("Error checking {}: {}", package.name, e);
                                    }
                                }
                            }
                        } else if cli.verbose && !cli.json {
                            println!(
                                "Skipping {} (conda): unable to extract channel URL",
                                package.name
                            );
                        }
                    } else if cli.verbose && !cli.json {
                        println!("Skipping {} (conda): no source URL", package.name);
                    }
                }
                pixi_outdated::pixi::PackageKind::Pypi => {
                    if cli.verbose && !cli.json {
                        println!("Checking {} (PyPI)...", package.name);
                    }

                    match pixi_outdated::pypi::get_latest_pypi_version(&package.name).await {
                        Ok(latest) => {
                            if latest != package.version {
                                let update = PackageUpdate {
                                    name: package.name.clone(),
                                    installed_version: package.version.clone(),
                                    latest_version: latest,
                                };
                                platform_package_updates.push(update);
                            } else if cli.verbose && !cli.json {
                                println!("{}: {} (up to date)", package.name, package.version);
                            }
                        }
                        Err(e) => {
                            if !cli.json {
                                eprintln!("Error checking {}: {}", package.name, e);
                            }
                        }
                    }
                }
            }

            // Increment progress bar
            if let Some(ref pb) = progress_bar {
                pb.inc(1);
            }
        }

        // Finish progress bar
        if let Some(ref pb) = progress_bar {
            pb.finish_with_message("Done");
        }

        // Store updates for this platform
        platform_updates.insert(platform.to_string(), platform_package_updates);
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
