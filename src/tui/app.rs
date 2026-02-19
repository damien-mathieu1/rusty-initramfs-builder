use crate::tui::screens::{ImageScreen, LanguageScreen};
use anyhow::Result;
use initramfs_builder::{BuildResult, Compression, InitramfsBuilder, RegistryAuth};
use tokio::sync::mpsc::{self, error::TryRecvError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Language,
    Image,
    Summary,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardMode {
    #[default]
    Quick,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub image: String,
    pub arch: String,
    pub compression: Compression,
    pub output: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            image: String::new(),
            arch: "amd64".to_string(),
            compression: Compression::Gzip,
            output: "initramfs.cpio.gz".to_string(),
        }
    }
}

pub struct App {
    pub screen: Screen,
    pub config: BuildConfig,
    #[allow(dead_code)]
    pub mode: WizardMode,
    pub should_quit: bool,
    pub language_screen: LanguageScreen,
    pub image_screen: ImageScreen,
    pub build_progress: Option<String>,
    pub build_error: Option<String>,
    pub validation_error: Option<String>,
    pub loading_frame: usize,
    pub build_success: bool,
    pub build_receiver: Option<mpsc::Receiver<Result<BuildResult>>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Language,
            config: BuildConfig::default(),
            mode: WizardMode::Quick,
            should_quit: false,
            language_screen: LanguageScreen::new(),
            image_screen: ImageScreen::new(),
            build_progress: None,
            build_error: None,
            validation_error: None,
            loading_frame: 0,
            build_success: false,
            build_receiver: None,
        }
    }

    pub fn next_screen(&mut self) {
        self.validation_error = None;
        self.sync_screen_on_exit();

        self.screen = match self.screen {
            Screen::Language => {
                self.update_image_from_language();
                Screen::Image
            }
            Screen::Image => Screen::Summary,
            Screen::Summary => Screen::Build,
            Screen::Build => Screen::Build,
        };

        self.sync_screen_on_enter();
    }

    pub fn prev_screen(&mut self) {
        self.validation_error = None;
        self.screen = match self.screen {
            Screen::Language => Screen::Language,
            Screen::Image => Screen::Language,
            Screen::Summary => Screen::Image,
            Screen::Build => Screen::Summary,
        };
        self.sync_screen_on_enter();
    }

    pub fn sync_screen_on_enter(&mut self) {
        if let Screen::Image = self.screen {
            self.image_screen.sync_from_config(&self.config.image);
        }
    }

    pub fn sync_screen_on_exit(&mut self) {
        if let Screen::Image = self.screen {
            self.config.image = self.image_screen.sync_to_config();
        }
    }

    fn update_image_from_language(&mut self) {
        let preset = &self.language_screen.presets[self.language_screen.selected];
        if !preset.versions.is_empty() {
            let version_idx = self
                .language_screen
                .version_selected
                .min(preset.versions.len() - 1);
            self.config.image = preset.versions[version_idx].1.to_string();
        }
    }

    pub fn validate_current_screen(&mut self) -> bool {
        self.validation_error = None;
        match self.screen {
            Screen::Image => {
                let image = self.image_screen.input.trim();
                if image.is_empty() {
                    self.validation_error = Some("Image cannot be empty".to_string());
                    return false;
                }
            }
            Screen::Summary => {
                if self.config.image.trim().is_empty() {
                    self.validation_error = Some("Image is required".to_string());
                    return false;
                }
            }
            _ => {}
        }
        true
    }

    pub fn start_build(&mut self) {
        self.screen = Screen::Build;
        self.build_progress = Some("Building initramfs...".to_string());
        self.loading_frame = 0;

        let image = self.config.image.clone();
        let arch = self.config.arch.clone();
        let compression = self.config.compression;
        let output = self.config.output.clone();
        let (tx, rx) = mpsc::channel::<Result<BuildResult>>(1);

        self.build_receiver = Some(rx);

        tokio::spawn(async move {
            let builder = InitramfsBuilder::new()
                .image(&image)
                .compression(compression)
                .platform("linux", &arch)
                .auth(RegistryAuth::Anonymous);

            let result = builder.build(&output).await;
            let _ = tx.send(result).await;
        });
    }

    pub fn check_build_status(&mut self) {
        if let Some(rx) = &mut self.build_receiver {
            let rx: &mut mpsc::Receiver<Result<BuildResult>> = rx;
            match rx.try_recv() {
                Ok(result) => {
                    self.build_receiver = None;
                    match result {
                        Ok(res) => {
                            self.build_progress = Some(format!(
                                "Success! Output: {} ({} entries, {:.2} MB)",
                                self.config.output,
                                res.entries,
                                res.compressed_size as f64 / 1_048_576.0
                            ));
                            self.build_success = true;
                        }
                        Err(e) => {
                            self.build_error = Some(format!("Build failed: {}", e));
                            self.build_success = false;
                        }
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.build_receiver = None;
                    self.build_error =
                        Some("Build task panicked or disconnected unexpectedly".to_string());
                    self.build_success = false;
                }
            }
        }
    }

    pub fn on_tick(&mut self) {
        self.loading_frame = self.loading_frame.wrapping_add(1);
    }

    pub fn generate_cli_command(&self) -> String {
        format!(
            "initramfs-builder build {} \\\n  --platform-arch {} \\\n  -c {} \\\n  -o {}",
            self.config.image, self.config.arch, self.config.compression, self.config.output
        )
    }
}
