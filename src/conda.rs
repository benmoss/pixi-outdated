use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rattler_conda_types::{
    Channel, ChannelConfig, MatchSpec, PackageName, Platform, VersionWithSource,
};
use rattler_repodata_gateway::Gateway;
use tracing::{debug, info};
use url::Url;

/// Global gateway instance that can be reused across queries
static GATEWAY: Lazy<Gateway> = Lazy::new(|| Gateway::builder().finish());

/// Extract the channel URL from a conda package source
/// Example: "https://conda.anaconda.org/conda-forge/" from package source
pub fn extract_channel_url(source: &str) -> Option<String> {
    if let Ok(url) = Url::parse(source) {
        // Get the base channel URL (scheme + host + first path segment)
        let channel_base = format!(
            "{}://{}/{}",
            url.scheme(),
            url.host_str()?,
            url.path_segments()?.next()?
        );
        Some(channel_base)
    } else {
        None
    }
}

/// Query conda channels for the latest version of a package across multiple platforms
pub async fn get_latest_conda_version_multi_platform(
    package_name: &str,
    channel_url: &str,
    platforms: &[&str],
) -> Result<Option<String>> {
    debug!(
        package = package_name,
        channel = channel_url,
        platforms = ?platforms,
        "Querying conda package across platforms"
    );

    let gateway = &*GATEWAY;

    // Parse the channel
    let channel_config = ChannelConfig::default_with_root_dir(std::env::current_dir()?);
    let channel = Channel::from_str(channel_url, &channel_config)
        .with_context(|| format!("Invalid channel URL: {}", channel_url))?;

    // Parse all platforms
    let mut parsed_platforms = vec![Platform::NoArch];
    for plat_str in platforms {
        let plat: Platform = plat_str
            .parse()
            .with_context(|| format!("Invalid platform: {}", plat_str))?;
        parsed_platforms.push(plat);
    }

    // Create a match spec for the package (any version)
    let package_name_typed = PackageName::try_from(package_name.to_string())
        .with_context(|| format!("Invalid package name: {}", package_name))?;

    let match_spec = MatchSpec::from_nameless(
        rattler_conda_types::NamelessMatchSpec {
            version: None,
            build: None,
            build_number: None,
            file_name: None,
            channel: None,
            subdir: None,
            namespace: None,
            md5: None,
            sha256: None,
            url: None,
            license: None,
            extras: None,
        },
        Some(package_name_typed),
    );

    let mut latest_version: Option<VersionWithSource> = None;

    // Query all platforms in a single call for efficiency
    let start = std::time::Instant::now();
    debug!(platforms = ?parsed_platforms, "Querying repodata");

    let records = gateway
        .query(
            vec![channel.clone()],
            parsed_platforms.clone(),
            vec![match_spec.clone()],
        )
        .await
        .with_context(|| format!("Failed to query channel {}", channel_url))?;

    let elapsed = start.elapsed();
    if elapsed.as_secs() > 1 {
        info!(
            package = package_name,
            elapsed_ms = elapsed.as_millis(),
            "Query completed (initial load)"
        );
    } else {
        debug!(
            package = package_name,
            elapsed_us = elapsed.as_micros(),
            "Query completed (cached)"
        );
    }

    // Process all records from all platforms
    for repo_data in records.iter() {
        for record in repo_data.iter() {
            let version = &record.package_record.version;

            match &latest_version {
                None => latest_version = Some(version.clone()),
                Some(current) => {
                    if version.version() > current.version() {
                        latest_version = Some(version.clone());
                    }
                }
            }
        }
    }

    Ok(latest_version.map(|v| v.version().to_string()))
}

/// Query conda channels for the latest version of a package
pub async fn get_latest_conda_version(
    package_name: &str,
    channel_url: &str,
    platform: &str,
) -> Result<Option<String>> {
    // Delegate to multi-platform version with a single platform
    get_latest_conda_version_multi_platform(package_name, channel_url, &[platform]).await
}
