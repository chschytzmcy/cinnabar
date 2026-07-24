use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_model_dir")]
    pub model_dir: String,

    // ----- ten-vad 字段 -----
    #[serde(default = "default_vad_model_path")]
    pub vad_model_path: String,
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
    #[serde(default = "default_vad_min_silence_ms")]
    pub vad_min_silence_ms: u32,
    #[serde(default = "default_vad_min_speech_ms")]
    pub vad_min_speech_ms: u32,
    #[serde(default = "default_vad_window_size")]
    pub vad_window_size: i32,
    #[serde(default = "default_vad_max_speech_duration")]
    pub vad_max_speech_duration: f32,
    #[serde(default = "default_vad_num_threads")]
    pub vad_num_threads: i32,
    #[serde(default = "default_vad_provider")]
    pub vad_provider: String,

    // ----- GUI 字段 -----
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
}

fn default_model_dir() -> String {
    "./models".to_string()
}

fn default_vad_model_path() -> String {
    "./models/ten-vad.onnx".to_string()
}

/// ten-vad 推荐的语音概率阈值起点；与原能量阈值 0.01 不可类比。
fn default_vad_threshold() -> f32 {
    0.5
}

fn default_vad_min_silence_ms() -> u32 {
    1200
}

fn default_vad_min_speech_ms() -> u32 {
    500
}

fn default_vad_window_size() -> i32 {
    256
}

fn default_vad_max_speech_duration() -> f32 {
    20.0
}

fn default_vad_num_threads() -> i32 {
    2
}

fn default_vad_provider() -> String {
    "cpu".to_string()
}

fn default_hotkey() -> String {
    "F3".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_dir: default_model_dir(),
            vad_model_path: default_vad_model_path(),
            vad_threshold: default_vad_threshold(),
            vad_min_silence_ms: default_vad_min_silence_ms(),
            vad_min_speech_ms: default_vad_min_speech_ms(),
            vad_window_size: default_vad_window_size(),
            vad_max_speech_duration: default_vad_max_speech_duration(),
            vad_num_threads: default_vad_num_threads(),
            vad_provider: default_vad_provider(),
            hotkey: default_hotkey(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.model_dir, "./models");
        // 注意：vad_threshold 现在是 ten-vad 概率阈值，默认 0.5；
        // 不再是早期能量 VAD 时代的 0.01。
        assert_eq!(config.vad_threshold, 0.5);
        assert_eq!(config.hotkey, "F3");
    }

    #[test]
    fn test_config_load_nonexistent() {
        let result = Config::load(&std::path::PathBuf::from("/nonexistent/config.toml"));
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.model_dir, "./models");
    }

    #[test]
    fn test_config_vad_fields_default() {
        let c = Config::default();
        assert_eq!(c.vad_model_path, "./models/ten-vad.onnx");
        assert_eq!(c.vad_threshold, 0.5);
        assert_eq!(c.vad_min_silence_ms, 1200);
        assert_eq!(c.vad_min_speech_ms, 500);
        assert_eq!(c.vad_window_size, 256);
        assert_eq!(c.vad_max_speech_duration, 20.0);
        assert_eq!(c.vad_num_threads, 2);
        assert_eq!(c.vad_provider, "cpu");
    }

    #[test]
    fn test_config_load_valid() {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp_file, "model_dir = \"/custom/models\"").unwrap();
        writeln!(temp_file, "vad_model_path = \"/custom/ten-vad.onnx\"").unwrap();
        writeln!(temp_file, "vad_threshold = 0.7").unwrap();
        writeln!(temp_file, "vad_min_silence_ms = 800").unwrap();
        writeln!(temp_file, "vad_min_speech_ms = 250").unwrap();
        writeln!(temp_file, "vad_window_size = 512").unwrap();
        writeln!(temp_file, "vad_num_threads = 4").unwrap();
        writeln!(temp_file, "vad_provider = \"cuda\"").unwrap();
        writeln!(temp_file, "hotkey = \"F4\"").unwrap();

        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.model_dir, "/custom/models");
        assert_eq!(config.vad_model_path, "/custom/ten-vad.onnx");
        assert_eq!(config.vad_threshold, 0.7);
        assert_eq!(config.vad_min_silence_ms, 800);
        assert_eq!(config.vad_min_speech_ms, 250);
        assert_eq!(config.vad_window_size, 512);
        assert_eq!(config.vad_num_threads, 4);
        assert_eq!(config.vad_provider, "cuda");
        assert_eq!(config.hotkey, "F4");
    }
}