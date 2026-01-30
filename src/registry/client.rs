use anyhow::{Context, Result};
use oci_distribution::{
    client::{Client, ClientConfig, ClientProtocol},
    manifest::OciDescriptor,
    secrets::RegistryAuth as OciRegistryAuth,
    Reference,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// Authentication credentials for a registry
#[derive(Debug, Clone, Default)]
pub enum RegistryAuth {
    #[default]
    Anonymous,
    Basic {
        username: String,
        password: String,
    },
}

impl From<RegistryAuth> for OciRegistryAuth {
    fn from(auth: RegistryAuth) -> Self {
        match auth {
            RegistryAuth::Anonymous => OciRegistryAuth::Anonymous,
            RegistryAuth::Basic { username, password } => {
                OciRegistryAuth::Basic(username, password)
            }
        }
    }
}

/// Options for pulling an image
#[derive(Debug, Clone)]
pub struct PullOptions {
    pub platform_os: String,
    pub platform_arch: String,
}

impl Default for PullOptions {
    fn default() -> Self {
        Self {
            platform_os: "linux".to_string(),
            platform_arch: "amd64".to_string(),
        }
    }
}

/// Describes a layer in an OCI image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDescriptor {
    pub digest: String,
    pub size: u64,
    pub media_type: String,
}

impl LayerDescriptor {
    fn to_oci_descriptor(&self) -> OciDescriptor {
        OciDescriptor {
            digest: self.digest.clone(),
            size: self.size as i64,
            media_type: self.media_type.clone(),
            ..Default::default()
        }
    }
}

/// Image manifest with layers info
#[derive(Debug, Clone)]
pub struct ImageManifest {
    pub config_digest: String,
    pub layers: Vec<LayerDescriptor>,
    pub total_size: u64,
}

/// Client for interacting with OCI registries
pub struct RegistryClient {
    client: Client,
    auth: RegistryAuth,
}

impl RegistryClient {
    pub fn new(auth: RegistryAuth) -> Self {
        let config = ClientConfig {
            protocol: ClientProtocol::Https,
            ..Default::default()
        };
        let client = Client::new(config);
        Self { client, auth }
    }

    pub fn parse_reference(image: &str) -> Result<Reference> {
        image
            .parse()
            .with_context(|| format!("Failed to parse image reference: {}", image))
    }

    /// Fetch the manifest for an image
    pub async fn fetch_manifest(
        &self,
        reference: &Reference,
        options: &PullOptions,
    ) -> Result<ImageManifest> {
        info!("Fetching manifest for {}", reference);

        let auth: OciRegistryAuth = self.auth.clone().into();

        let (manifest, _digest) = self
            .client
            .pull_manifest(reference, &auth)
            .await
            .with_context(|| format!("Failed to pull manifest for {}", reference))?;

        let oci_manifest = match manifest {
            oci_distribution::manifest::OciManifest::Image(m) => m,
            oci_distribution::manifest::OciManifest::ImageIndex(index) => {
                // Multi-arch image, find the right platform
                let platform_manifest = index
                    .manifests
                    .iter()
                    .find(|m| {
                        if let Some(p) = &m.platform {
                            p.os == options.platform_os && p.architecture == options.platform_arch
                        } else {
                            false
                        }
                    })
                    .with_context(|| {
                        format!(
                            "Platform {}/{} not found in image index",
                            options.platform_os, options.platform_arch
                        )
                    })?;

                debug!("Found platform manifest: {:?}", platform_manifest.digest);

                // Create a reference with the specific digest
                let platform_ref = Reference::with_digest(
                    reference.registry().to_string(),
                    reference.repository().to_string(),
                    platform_manifest.digest.clone(),
                );

                let (platform_manifest, _) = self
                    .client
                    .pull_manifest(&platform_ref, &auth)
                    .await
                    .with_context(|| "Failed to pull platform-specific manifest")?;

                match platform_manifest {
                    oci_distribution::manifest::OciManifest::Image(m) => m,
                    _ => anyhow::bail!("Expected image manifest, got index"),
                }
            }
        };

        let layers: Vec<LayerDescriptor> = oci_manifest
            .layers
            .iter()
            .map(|l| LayerDescriptor {
                digest: l.digest.clone(),
                size: l.size as u64,
                media_type: l.media_type.clone(),
            })
            .collect();

        let total_size = layers.iter().map(|l| l.size).sum();

        Ok(ImageManifest {
            config_digest: oci_manifest.config.digest.clone(),
            layers,
            total_size,
        })
    }

    /// Pull a specific layer and return its content as bytes
    pub async fn pull_layer(
        &self,
        reference: &Reference,
        layer: &LayerDescriptor,
    ) -> Result<Vec<u8>> {
        debug!("Pulling layer {} ({} bytes)", layer.digest, layer.size);

        let _auth: OciRegistryAuth = self.auth.clone().into();
        let descriptor = layer.to_oci_descriptor();

        // Create a buffer to receive the blob data
        let mut data = Vec::with_capacity(layer.size as usize);

        self.client
            .pull_blob(reference, &descriptor, &mut data)
            .await
            .with_context(|| format!("Failed to pull layer {}", layer.digest))?;

        Ok(data)
    }

    /// Pull all layers and return them in order
    pub async fn pull_all_layers(
        &self,
        reference: &Reference,
        manifest: &ImageManifest,
        progress_callback: Option<Arc<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<Vec<Vec<u8>>> {
        let mut layers_data = Vec::with_capacity(manifest.layers.len());
        let total = manifest.layers.len();

        for (idx, layer) in manifest.layers.iter().enumerate() {
            if let Some(ref cb) = progress_callback {
                cb(idx + 1, total);
            }
            let data = self.pull_layer(reference, layer).await?;
            layers_data.push(data);
        }

        Ok(layers_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reference_simple() {
        let reference = RegistryClient::parse_reference("alpine:latest").unwrap();
        assert_eq!(reference.repository(), "library/alpine");
        assert_eq!(reference.tag(), Some("latest"));
    }

    #[test]
    fn test_parse_reference_with_registry() {
        let reference = RegistryClient::parse_reference("ghcr.io/user/repo:v1").unwrap();
        assert_eq!(reference.registry(), "ghcr.io");
        assert_eq!(reference.repository(), "user/repo");
        assert_eq!(reference.tag(), Some("v1"));
    }
}
