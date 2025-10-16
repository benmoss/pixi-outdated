use anyhow::Result;
use serde::Deserialize;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct PyPiResponse {
    info: PyPiInfo,
}

#[derive(Debug, Deserialize)]
struct PyPiInfo {
    version: String,
}

/// Query PyPI for the latest version of a package
pub async fn get_latest_pypi_version(package_name: &str) -> Result<String> {
    debug!(package = package_name, "Querying PyPI package");

    let url = format!("https://pypi.org/pypi/{}/json", package_name);
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data: PyPiResponse = response.json().await?;
        let elapsed = start.elapsed();

        debug!(
            package = package_name,
            version = %data.info.version,
            elapsed_ms = elapsed.as_millis(),
            "PyPI query completed"
        );

        Ok(data.info.version)
    } else {
        anyhow::bail!(
            "Failed to fetch PyPI data for {}: {}",
            package_name,
            response.status()
        )
    }
}
