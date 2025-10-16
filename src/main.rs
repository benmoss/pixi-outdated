use anyhow::Result;
use clap::Parser;

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

    /// The platform to check (defaults to the current platform)
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

async fn run(cli: Cli) -> Result<()> {
    // Step 1: Get package list from `pixi list --json`
    if cli.verbose {
        println!("Fetching package list from pixi...");
    }

    let packages = pixi_outdated::pixi::get_package_list(
        cli.explicit,
        cli.environment.as_deref(),
        cli.platform.as_deref(),
        cli.manifest.as_deref(),
        &cli.packages,
    )?;

    if cli.verbose {
        println!("Found {} packages\n", packages.len());
    }

    // Step 2: Query for latest versions
    if cli.verbose {
        println!("Querying for latest versions...\n");
    }

    let platform = cli.platform.as_deref().unwrap_or("osx-arm64"); // TODO: Get from system

    for package in &packages {
        match package.kind {
            pixi_outdated::pixi::PackageKind::Conda => {
                // Extract channel URL from the source
                if let Some(ref source) = package.source {
                    if let Some(channel_url) = pixi_outdated::conda::extract_channel_url(source) {
                        if cli.verbose {
                            println!("Checking {} (conda) from {}...", package.name, channel_url);
                        }

                        match pixi_outdated::conda::get_latest_conda_version(
                            &package.name,
                            &channel_url,
                            platform,
                        )
                        .await
                        {
                            Ok(Some(latest)) => {
                                if latest != package.version {
                                    println!("{}: {} -> {}", package.name, package.version, latest);
                                } else if cli.verbose {
                                    println!("{}: {} (up to date)", package.name, package.version);
                                }
                            }
                            Ok(None) => {
                                if cli.verbose {
                                    println!(
                                        "{}: {} (no newer version found)",
                                        package.name, package.version
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("Error checking {}: {}", package.name, e);
                            }
                        }
                    } else if cli.verbose {
                        println!(
                            "Skipping {} (conda): unable to extract channel URL",
                            package.name
                        );
                    }
                } else if cli.verbose {
                    println!("Skipping {} (conda): no source URL", package.name);
                }
            }
            pixi_outdated::pixi::PackageKind::Pypi => {
                if cli.verbose {
                    println!("Checking {} (PyPI)...", package.name);
                }

                match pixi_outdated::pypi::get_latest_pypi_version(&package.name).await {
                    Ok(latest) => {
                        if latest != package.version {
                            println!("{}: {} -> {}", package.name, package.version, latest);
                        } else if cli.verbose {
                            println!("{}: {} (up to date)", package.name, package.version);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking {}: {}", package.name, e);
                    }
                }
            }
        }
    }

    Ok(())
}
