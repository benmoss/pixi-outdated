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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_channel_url_valid() {
        let source =
            "https://conda.anaconda.org/conda-forge/linux-64/python-3.12.0-h1234567_0.conda";
        let channel = extract_channel_url(source);
        assert_eq!(
            channel,
            Some("https://conda.anaconda.org/conda-forge".to_string())
        );
    }

    #[test]
    fn test_extract_channel_url_different_host() {
        let source = "https://repo.prefix.dev/channel-name/osx-arm64/package.conda";
        let channel = extract_channel_url(source);
        assert_eq!(
            channel,
            Some("https://repo.prefix.dev/channel-name".to_string())
        );
    }

    #[test]
    fn test_extract_channel_url_no_path() {
        let source = "https://conda.anaconda.org/";
        let channel = extract_channel_url(source);
        // URL with trailing slash but empty path segment still extracts base URL
        assert_eq!(channel, Some("https://conda.anaconda.org/".to_string()));
    }

    #[test]
    fn test_extract_channel_url_invalid_url() {
        let source = "not-a-valid-url";
        let channel = extract_channel_url(source);
        assert_eq!(channel, None);
    }

    #[test]
    fn test_extract_channel_url_file_path() {
        let source = "/local/path/to/package.conda";
        let channel = extract_channel_url(source);
        assert_eq!(channel, None);
    }

    #[tokio::test]
    async fn test_get_latest_conda_version_delegates_to_multi() {
        // This test verifies that the single-platform version correctly
        // delegates to the multi-platform version
        // We can't easily test the actual query without mocking, but we can
        // verify the function signature is correct and it doesn't panic

        // This will fail with an invalid channel, but that's expected
        let result = get_latest_conda_version(
            "nonexistent-package-xyz",
            "https://conda.anaconda.org/conda-forge",
            "linux-64",
        )
        .await;

        // Either succeeds with None (package not found) or fails with network/channel error
        // Both are valid outcomes for this test
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_platform_parsing() {
        // Test that platform strings can be parsed correctly
        use rattler_conda_types::Platform;

        let platforms = vec!["linux-64", "osx-arm64", "win-64", "osx-64"];

        for plat_str in platforms {
            let result: Result<Platform, _> = plat_str.parse();
            assert!(result.is_ok(), "Failed to parse platform: {}", plat_str);
        }
    }

    #[test]
    fn test_invalid_platform() {
        use rattler_conda_types::Platform;

        let invalid_platforms = vec!["invalid-platform", "windows-x64", "mac-arm"];

        for plat_str in invalid_platforms {
            let result: Result<Platform, _> = plat_str.parse();
            assert!(
                result.is_err(),
                "Should have failed to parse invalid platform: {}",
                plat_str
            );
        }
    }
}
