use crate::tui::screens::{
    ArchScreen, CompressScreen, ImageScreen, InitScreen, InjectScreen, LanguageScreen,
};
use crate::{Compression, InitramfsBuilder, RegistryAuth};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Language,
    Image,
    Architecture,
    Inject,
    Init,
    Compression,
    Summary,
    Building,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardMode {
    #[default]
    Quick,
    Advanced,
}

#[derive(Debug, Clone)]
pub enum InitMode {
    Default,
    CustomFile(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Injection {
    pub src: String,
    pub dest: String,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub image: String,
    pub arch: String,
    pub injections: Vec<Injection>,
    pub init_mode: InitMode,
    pub compression: Compression,
    pub output: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            image: String::new(),
            arch: detect_host_arch().to_string(),
            injections: Vec::new(),
            init_mode: InitMode::Default,
            compression: Compression::Gzip,
            output: "initramfs.cpio.gz".to_string(),
        }
    }
}

fn detect_host_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => "amd64",
    }
}

pub struct App {
    pub screen: Screen,
    pub config: BuildConfig,
    pub mode: WizardMode,
    pub should_quit: bool,
    pub language_screen: LanguageScreen,
    pub image_screen: ImageScreen,
    pub arch_screen: ArchScreen,
    pub inject_screen: InjectScreen,
    pub init_screen: InitScreen,
    pub compress_screen: CompressScreen,
    pub build_progress: Option<String>,
    pub build_error: Option<String>,
    pub validation_error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let config = BuildConfig::default();
        let arch_screen = ArchScreen::new_with_default(&config.arch);
        Self {
            screen: Screen::Language,
            config,
            mode: WizardMode::Quick,
            should_quit: false,
            language_screen: LanguageScreen::new(),
            image_screen: ImageScreen::new(),
            arch_screen,
            inject_screen: InjectScreen::new(),
            init_screen: InitScreen::new(),
            compress_screen: CompressScreen::new(),
            build_progress: None,
            build_error: None,
            validation_error: None,
        }
    }

    pub fn sync_screen_on_enter(&mut self) {
        match self.screen {
            Screen::Image => {
                self.image_screen.sync_from_config(&self.config.image);
            }
            Screen::Architecture => {
                self.arch_screen.sync_from_config(&self.config.arch);
            }
            _ => {}
        }
    }

    pub fn sync_screen_on_exit(&mut self) {
        match self.screen {
            Screen::Image => {
                self.config.image = self.image_screen.sync_to_config();
            }
            Screen::Architecture => {
                self.config.arch = self.arch_screen.get_selected().to_string();
            }
            Screen::Inject => {
                self.config.injections = self.inject_screen.get_injections();
            }
            Screen::Init => {
                self.config.init_mode = self.init_screen.get_init_mode();
            }
            Screen::Compression => {
                self.config.compression = self.compress_screen.get_selected();
            }
            _ => {}
        }
    }

    pub fn is_config_valid(&self) -> bool {
        !self.config.image.trim().is_empty()
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
                if !self.is_config_valid() {
                    self.validation_error =
                        Some("Configuration invalid: image is required".to_string());
                    return false;
                }
            }
            _ => {}
        }
        true
    }

    pub fn next_screen(&mut self) {
        self.sync_screen_on_exit();

        self.screen = match self.screen {
            Screen::Language => {
                self.update_image_from_language();
                Screen::Image
            }
            Screen::Image => match self.mode {
                WizardMode::Quick => Screen::Summary,
                WizardMode::Advanced => Screen::Architecture,
            },
            Screen::Architecture => Screen::Inject,
            Screen::Inject => Screen::Init,
            Screen::Init => Screen::Compression,
            Screen::Compression => Screen::Summary,
            Screen::Summary => Screen::Building,
            Screen::Building => Screen::Building,
        };

        self.sync_screen_on_enter();
    }

    pub fn prev_screen(&mut self) {
        self.validation_error = None;
        self.screen = match self.screen {
            Screen::Language => Screen::Language,
            Screen::Image => Screen::Language,
            Screen::Architecture => Screen::Image,
            Screen::Inject => Screen::Architecture,
            Screen::Init => Screen::Inject,
            Screen::Compression => Screen::Init,
            Screen::Summary => match self.mode {
                WizardMode::Quick => Screen::Image,
                WizardMode::Advanced => Screen::Compression,
            },
            Screen::Building => Screen::Summary,
        };
        self.sync_screen_on_enter();
    }

    pub fn enter_advanced_mode(&mut self) {
        self.mode = WizardMode::Advanced;
        self.screen = Screen::Architecture;
        self.sync_screen_on_enter();
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

    pub async fn execute_build(&mut self) -> Result<()> {
        self.build_progress = Some("Building initramfs...".to_string());

        let mut builder = InitramfsBuilder::new()
            .image(&self.config.image)
            .compression(self.config.compression)
            .platform("linux", &self.config.arch)
            .auth(RegistryAuth::Anonymous);

        for inj in &self.config.injections {
            builder = builder.inject(&inj.src, &inj.dest);
        }

        if let InitMode::CustomFile(path) = &self.config.init_mode {
            builder = builder.init_script(path.clone());
        }

        match builder.build(&self.config.output).await {
            Ok(result) => {
                self.build_progress = Some(format!(
                    "Success! Output: {} ({} entries, {:.2} MB)",
                    self.config.output,
                    result.entries,
                    result.compressed_size as f64 / 1_048_576.0
                ));
            }
            Err(e) => {
                self.build_error = Some(format!("Build failed: {}", e));
            }
        }

        Ok(())
    }

    pub fn generate_cli_command(&self) -> String {
        let mut cmd = format!("initramfs-builder build {}", self.config.image);

        for inj in &self.config.injections {
            cmd.push_str(&format!(" \\\n  --inject {}:{}", inj.src, inj.dest));
        }

        if let InitMode::CustomFile(path) = &self.config.init_mode {
            cmd.push_str(&format!(" \\\n  --init {}", path.display()));
        }

        cmd.push_str(&format!(" \\\n  --platform-arch {}", self.config.arch));
        cmd.push_str(&format!(" \\\n  -c {}", self.config.compression));
        cmd.push_str(&format!(" \\\n  -o {}", self.config.output));

        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_sets_config_image() {
        let mut app = App::new();
        app.language_screen.selected = 0;
        app.language_screen.version_selected = 0;
        app.update_image_from_language();
        assert_eq!(app.config.image, "python:3.12-alpine");
    }

    #[test]
    fn test_image_validation_blocks_empty() {
        let mut app = App::new();
        app.screen = Screen::Image;
        app.image_screen.input = "".to_string();
        assert!(!app.validate_current_screen());
        assert!(app.validation_error.is_some());
    }

    #[test]
    fn test_host_arch_detection() {
        let arch = detect_host_arch();
        assert!(arch == "amd64" || arch == "arm64");
    }

    #[test]
    fn test_quick_mode_skips_advanced_screens() {
        let mut app = App::new();
        app.mode = WizardMode::Quick;
        app.screen = Screen::Image;
        app.image_screen.input = "test:image".to_string();
        app.next_screen();
        assert_eq!(app.screen, Screen::Summary);
    }
}
