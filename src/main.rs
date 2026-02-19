use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use initramfs_builder::{Compression, InitramfsBuilder, RegistryAuth, RegistryClient};
use std::io::{self, BufRead};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "initramfs-builder")]
#[command(author, version, about = "Convert Docker/OCI images to bootable initramfs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

mod tui;

#[derive(Subcommand)]
enum Commands {
    /// Build an initramfs from a Docker/OCI image
    Build {
        /// Image reference (e.g., python:3.11-alpine)
        image: String,

        /// Output file path
        #[arg(short, long, default_value = "initramfs.cpio.gz")]
        output: String,

        /// Compression format (gzip, zstd, none)
        #[arg(short, long, default_value = "gzip")]
        compression: String,

        /// Patterns to exclude (can be repeated)
        #[arg(long)]
        exclude: Vec<String>,

        /// Inject files into initramfs (format: /path/on/host:/path/in/initramfs)
        #[arg(long, value_name = "SRC:DEST")]
        inject: Vec<String>,

        /// Custom init script to use (will be placed at /init)
        #[arg(long, value_name = "PATH")]
        init: Option<PathBuf>,

        /// Target platform OS
        #[arg(long, default_value = "linux")]
        platform_os: String,

        /// Target platform architecture
        #[arg(long, default_value = "amd64")]
        platform_arch: String,

        /// Registry username
        #[arg(long)]
        username: Option<String>,

        /// Read password from stdin
        #[arg(long)]
        password_stdin: bool,
    },

    /// Inspect an image (show manifest info)
    Inspect {
        /// Image reference
        image: String,

        /// Target platform OS
        #[arg(long, default_value = "linux")]
        platform_os: String,

        /// Target platform architecture
        #[arg(long, default_value = "amd64")]
        platform_arch: String,
    },

    /// List layers of an image
    ListLayers {
        /// Image reference
        image: String,

        /// Target platform OS
        #[arg(long, default_value = "linux")]
        platform_os: String,

        /// Target platform architecture
        #[arg(long, default_value = "amd64")]
        platform_arch: String,
    },

    /// Interactive mode (TUI)
    Interactive,
}

fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn read_password_stdin() -> Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Parse inject argument in format "src:dest"
fn parse_inject(s: &str) -> Result<(PathBuf, PathBuf)> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid inject format '{}'. Expected format: /path/on/host:/path/in/initramfs",
            s
        );
    }
    Ok((PathBuf::from(parts[0]), PathBuf::from(parts[1])))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            image,
            output,
            compression,
            exclude,
            inject,
            init,
            platform_os,
            platform_arch,
            username,
            password_stdin,
        } => {
            setup_logging(cli.verbose);
            let compression: Compression = compression
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;

            let auth = match (username, password_stdin) {
                (Some(user), true) => {
                    let password = read_password_stdin()?;
                    RegistryAuth::Basic {
                        username: user,
                        password,
                    }
                }
                (Some(user), false) => {
                    eprintln!("Warning: username provided without password");
                    RegistryAuth::Basic {
                        username: user,
                        password: String::new(),
                    }
                }
                _ => RegistryAuth::Anonymous,
            };

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message(format!("Building initramfs from {}...", image));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let exclude_refs: Vec<&str> = exclude.iter().map(|s| s.as_str()).collect();

            let mut builder = InitramfsBuilder::new()
                .image(&image)
                .compression(compression)
                .platform(&platform_os, &platform_arch)
                .auth(auth);

            for pattern in &exclude_refs {
                builder = builder.exclude(&[*pattern]);
            }

            for inject_arg in &inject {
                let (src, dest) = parse_inject(inject_arg)?;
                builder = builder.inject(src, dest);
            }

            if let Some(init_path) = init {
                builder = builder.init_script(init_path);
            }

            let result = builder.build(&output).await?;

            pb.finish_and_clear();

            println!("Successfully built initramfs:");
            println!("  Output: {}", output);
            println!("  Entries: {}", result.entries);
            println!("  Uncompressed: {}", format_size(result.uncompressed_size));
            println!("  Compressed: {}", format_size(result.compressed_size));
            println!(
                "  Ratio: {:.1}%",
                (result.compressed_size as f64 / result.uncompressed_size as f64) * 100.0
            );
            if result.injected_files > 0 {
                println!("  Injected files: {}", result.injected_files);
            }
            if result.has_custom_init {
                println!("  Custom init: yes");
            }
        }

        Commands::Inspect {
            image,
            platform_os,
            platform_arch,
        } => {
            setup_logging(cli.verbose);
            let client = RegistryClient::new(RegistryAuth::Anonymous);
            let reference = RegistryClient::parse_reference(&image)?;
            let options = initramfs_builder::PullOptions {
                platform_os,
                platform_arch,
            };

            let manifest = client.fetch_manifest(&reference, &options).await?;

            println!("Image: {}", image);
            println!("Config digest: {}", manifest.config_digest);
            println!("Layers: {}", manifest.layers.len());
            println!("Total size: {}", format_size(manifest.total_size));
        }

        Commands::ListLayers {
            image,
            platform_os,
            platform_arch,
        } => {
            setup_logging(cli.verbose);
            let client = RegistryClient::new(RegistryAuth::Anonymous);
            let reference = RegistryClient::parse_reference(&image)?;
            let options = initramfs_builder::PullOptions {
                platform_os,
                platform_arch,
            };

            let manifest = client.fetch_manifest(&reference, &options).await?;

            println!("Layers for {}:", image);
            println!();
            for (idx, layer) in manifest.layers.iter().enumerate() {
                println!(
                    "  {}. {} ({})",
                    idx + 1,
                    &layer.digest[7..19],
                    format_size(layer.size)
                );
            }
            println!();
            println!("{}", format_size(manifest.total_size));
        }

        Commands::Interactive => {
            tui::run().await?;
        }
    }

    Ok(())
}
