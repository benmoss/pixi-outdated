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
    /// Specific package name to check (if not provided, checks all packages)
    package_name: Option<String>,

    /// Only check packages explicitly listed in pixi.toml (not transitive dependencies)
    #[arg(short, long)]
    explicit: bool,

    /// Output in JSON format
    #[arg(short, long)]
    json: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Path to the pixi.toml file (defaults to current directory)
    #[arg(short = 'f', long, default_value = "pixi.toml")]
    manifest: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Running pixi-outdated with options:");
        println!("  Manifest: {}", cli.manifest);
        println!("  Explicit only: {}", cli.explicit);
        println!("  JSON output: {}", cli.json);
        if let Some(ref pkg) = cli.package_name {
            println!("  Checking package: {}", pkg);
        } else {
            println!("  Checking all packages");
        }
        println!();
    }

    // TODO: Implement the actual logic
    run(cli).await
}

async fn run(cli: Cli) -> Result<()> {
    // Step 1: Parse pixi.toml to get declared dependencies
    println!("Reading manifest from: {}", cli.manifest);

    // Step 2: Parse pixi.lock to get currently locked versions
    let lock_path = cli.manifest.replace("pixi.toml", "pixi.lock");
    println!("Reading lockfile from: {}", lock_path);

    // Step 3: Query repodata for conda packages
    // TODO: Use rattler to query conda channels

    // Step 4: Query PyPI for PyPI packages
    // TODO: Query PyPI JSON API

    // Step 5: Compare versions and output results
    // TODO: Implement version comparison

    println!("\nAnalysis complete! (not yet implemented)");

    Ok(())
}
