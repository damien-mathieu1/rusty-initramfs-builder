use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    Gzip,
    Zstd,
    None,
}

impl std::str::FromStr for Compression {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gzip" | "gz" => Ok(Compression::Gzip),
            "zstd" | "zst" => Ok(Compression::Zstd),
            "none" | "raw" => Ok(Compression::None),
            _ => Err(format!("Unknown compression: {}", s)),
        }
    }
}

impl std::fmt::Display for Compression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compression::Gzip => write!(f, "gzip"),
            Compression::Zstd => write!(f, "zstd"),
            Compression::None => write!(f, "none"),
        }
    }
}

/// Compress data and write to output path
pub fn compress_archive(data: &[u8], output_path: &Path, compression: Compression) -> Result<u64> {
    info!(
        "Compressing {} bytes with {} to {:?}",
        data.len(),
        compression,
        output_path
    );

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {:?}", output_path))?;
    let mut writer = BufWriter::new(file);

    match compression {
        Compression::Gzip => {
            let mut encoder = GzEncoder::new(&mut writer, GzCompression::default());
            encoder.write_all(data)?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let mut encoder = zstd::stream::Encoder::new(&mut writer, 3)?;
            encoder.write_all(data)?;
            encoder.finish()?;
        }
        Compression::None => {
            writer.write_all(data)?;
        }
    }

    writer.flush()?;

    let output_size = std::fs::metadata(output_path)?.len();
    info!(
        "Compressed {} bytes -> {} bytes ({:.1}% ratio)",
        data.len(),
        output_size,
        (output_size as f64 / data.len() as f64) * 100.0
    );

    Ok(output_size)
}
