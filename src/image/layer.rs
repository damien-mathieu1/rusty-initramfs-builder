use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::debug;

pub struct LayerExtractor {
    exclude_patterns: Vec<glob::Pattern>,
    whiteouts: HashSet<PathBuf>,
    opaque_dirs: HashSet<PathBuf>,
}

impl LayerExtractor {
    pub fn new() -> Self {
        Self {
            exclude_patterns: Vec::new(),
            whiteouts: HashSet::new(),
            opaque_dirs: HashSet::new(),
        }
    }

    pub fn with_excludes(mut self, patterns: &[&str]) -> Result<Self> {
        for pattern in patterns {
            let compiled = glob::Pattern::new(pattern)
                .with_context(|| format!("Invalid glob pattern: {}", pattern))?;
            self.exclude_patterns.push(compiled);
        }
        Ok(self)
    }

    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exclude_patterns
            .iter()
            .any(|p| p.matches(&path_str) || p.matches_path(path))
    }

    /// Extract a single layer (gzipped tar) to the target directory
    pub fn extract_layer(&mut self, layer_data: &[u8], target_dir: &Path) -> Result<()> {
        // First pass: collect whiteouts
        let decoder = GzDecoder::new(layer_data);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?;

            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();

                if name_str == ".wh..wh..opq" {
                    if let Some(parent) = path.parent() {
                        debug!("Opaque whiteout for directory: {:?}", parent);
                        self.opaque_dirs.insert(parent.to_path_buf());

                        // Remove existing directory contents
                        let full_path = target_dir.join(parent);
                        if full_path.exists() {
                            fs::remove_dir_all(&full_path).ok();
                            fs::create_dir_all(&full_path)?;
                        }
                    }
                } else if name_str.starts_with(".wh.") {
                    let deleted_name = name_str.strip_prefix(".wh.").unwrap();
                    let deleted_path = path
                        .parent()
                        .map_or_else(|| PathBuf::from(deleted_name), |p| p.join(deleted_name));
                    debug!("Whiteout for file: {:?}", deleted_path);
                    self.whiteouts.insert(deleted_path.to_path_buf());

                    let full_path = target_dir.join(&deleted_path);
                    if full_path.exists() {
                        if full_path.is_dir() {
                            fs::remove_dir_all(&full_path).ok();
                        } else {
                            fs::remove_file(&full_path).ok();
                        }
                    }
                }
            }
        }

        // Second pass: extract files with proper handling
        let decoder2 = GzDecoder::new(layer_data);
        let mut archive2 = Archive::new(decoder2);
        archive2.set_preserve_permissions(true);
        archive2.set_preserve_mtime(true);
        // Don't preserve ownership on extraction (we're not root)
        archive2.set_unpack_xattrs(false);

        for entry in archive2.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let path_owned = path.to_path_buf();

            // Skip whiteout marker files
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with(".wh.") {
                    continue;
                }
            }

            // Skip excluded paths
            if self.should_exclude(&path_owned) {
                debug!("Excluding: {:?}", path_owned);
                continue;
            }

            let target_path = target_dir.join(&path_owned);

            // Ensure parent directory exists
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Handle different entry types
            let entry_type = entry.header().entry_type();

            match entry_type {
                tar::EntryType::Link => {
                    // Hard link - get the link target and copy instead
                    if let Ok(link_name) = entry.link_name() {
                        if let Some(link_target) = link_name {
                            let source_path = target_dir.join(link_target.as_ref());
                            if source_path.exists() {
                                // Try hard link first, fall back to copy
                                if fs::hard_link(&source_path, &target_path).is_err() {
                                    fs::copy(&source_path, &target_path).ok();
                                }
                            }
                        }
                    }
                }
                tar::EntryType::Symlink => {
                    // Symlink - create it
                    if let Ok(link_name) = entry.link_name() {
                        if let Some(link_target) = link_name {
                            // Remove existing file if any
                            if target_path.exists() || target_path.is_symlink() {
                                fs::remove_file(&target_path).ok();
                            }
                            #[cfg(unix)]
                            std::os::unix::fs::symlink(link_target.as_ref(), &target_path).ok();
                        }
                    }
                }
                _ => {
                    // Regular file or directory - use normal unpack
                    entry
                        .unpack(&target_path)
                        .with_context(|| format!("Failed to extract {:?}", path_owned))?;
                }
            }
        }

        Ok(())
    }

    /// Extract all layers in order to build the final rootfs
    pub fn extract_all_layers(&mut self, layers: &[Vec<u8>], target_dir: &Path) -> Result<()> {
        fs::create_dir_all(target_dir)?;

        for (idx, layer_data) in layers.iter().enumerate() {
            debug!("Extracting layer {}/{}", idx + 1, layers.len());
            self.extract_layer(layer_data, target_dir)?;
        }

        Ok(())
    }
}

impl Default for LayerExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclude_patterns() {
        let extractor = LayerExtractor::new()
            .with_excludes(&["/usr/share/doc/*", "*.pyc"])
            .unwrap();

        assert!(extractor.should_exclude(Path::new("/usr/share/doc/readme.txt")));
        assert!(extractor.should_exclude(Path::new("module.pyc")));
        assert!(!extractor.should_exclude(Path::new("/usr/bin/python")));
    }
}
