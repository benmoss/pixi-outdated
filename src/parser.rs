use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct PixiManifest {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "pypi-dependencies", default)]
    pub pypi_dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub channels: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PixiLock {
    pub version: u32,
    pub environments: HashMap<String, Environment>,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Deserialize)]
pub struct Environment {
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub indexes: Vec<String>,
    pub packages: HashMap<String, Vec<PackageRef>>,
}

#[derive(Debug, Deserialize)]
pub struct Channel {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PackageRef {
    #[serde(flatten)]
    pub source: PackageSource,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PackageSource {
    Conda { conda: String },
    PyPI { pypi: String },
}

#[derive(Debug, Deserialize)]
pub struct LockedPackage {
    #[serde(flatten)]
    pub source: PackageSource,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

pub fn parse_manifest(path: &Path) -> Result<PixiManifest> {
    let content = fs::read_to_string(path)?;
    let manifest: PixiManifest = toml::from_str(&content)?;
    Ok(manifest)
}

pub fn parse_lockfile(path: &Path) -> Result<PixiLock> {
    let content = fs::read_to_string(path)?;
    let lockfile: PixiLock = serde_yaml::from_str(&content)?;
    Ok(lockfile)
}
