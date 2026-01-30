use anyhow::Result;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::info;

use super::LayerExtractor;
use crate::registry::{PullOptions, RegistryClient};

pub struct RootfsBuilder {
    client: RegistryClient,
    options: PullOptions,
    exclude_patterns: Vec<String>,
    temp_dir: Option<TempDir>,
}

impl RootfsBuilder {
    pub fn new(client: RegistryClient) -> Self {
        Self {
            client,
            options: PullOptions::default(),
            exclude_patterns: Vec::new(),
            temp_dir: None,
        }
    }

    pub fn platform(mut self, os: &str, arch: &str) -> Self {
        self.options.platform_os = os.to_string();
        self.options.platform_arch = arch.to_string();
        self
    }

    pub fn exclude(mut self, patterns: &[&str]) -> Self {
        self.exclude_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    pub async fn build(&mut self, image: &str) -> Result<PathBuf> {
        let reference = RegistryClient::parse_reference(image)?;

        info!("Fetching manifest for {}", image);
        let manifest = self
            .client
            .fetch_manifest(&reference, &self.options)
            .await?;

        info!(
            "Image has {} layers, total size: {} bytes",
            manifest.layers.len(),
            manifest.total_size
        );

        info!("Pulling layers...");
        let layers = self
            .client
            .pull_all_layers(&reference, &manifest, None)
            .await?;

        let temp_dir = TempDir::new()?;
        let rootfs_path = temp_dir.path().to_path_buf();

        info!("Extracting layers to {:?}", rootfs_path);
        let exclude_refs: Vec<&str> = self.exclude_patterns.iter().map(|s| s.as_str()).collect();
        let mut extractor = LayerExtractor::new().with_excludes(&exclude_refs)?;
        extractor.extract_all_layers(&layers, &rootfs_path)?;

        self.temp_dir = Some(temp_dir);

        Ok(rootfs_path)
    }

    pub fn rootfs_path(&self) -> Option<&Path> {
        self.temp_dir.as_ref().map(|t| t.path())
    }
}
