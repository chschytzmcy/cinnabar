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

    // ----- 离线 ASR refine 字段 -----
    #[serde(default = "default_offline_model_path")]
    pub offline_model_path: String,
    #[serde(default = "default_offline_tokens_path")]
    pub offline_tokens_path: String,
    #[serde(default = "default_offline_num_threads")]
    pub offline_num_threads: i32,
    #[serde(default = "default_offline_provider")]
    pub offline_provider: String,
    #[serde(default = "default_offline_decoding")]
    pub offline_decoding: String,
    #[serde(default = "default_offline_hotwords_file")]
    pub offline_hotwords_file: String,
    #[serde(default = "default_offline_hotwords_score")]
    pub offline_hotwords_score: f32,
    #[serde(default = "default_enable_offline_refine")]
    pub enable_offline_refine: bool,

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

// 离线 ASR 默认值：Paraformer zh-2023-09-14（中文 attention decoder，233MB int8）
// 选 Paraformer 因为 attention decoder 在 beam search 阶段对热词加权最稳定
// （CTC 框架的热词加成弱，SenseVoice 是 encoder-only 不支持 beam bias）
fn default_offline_model_path() -> String {
    "./models/sherpa-onnx-paraformer-zh-2023-09-14/model.int8.onnx".to_string()
}

fn default_offline_tokens_path() -> String {
    "./models/sherpa-onnx-paraformer-zh-2023-09-14/tokens.txt".to_string()
}

/// 热词文件路径；空字符串表示关闭热词功能
/// 默认指向 `docs/hotword.md`（仓库自带的人名/地名/公司名热词清单）
fn default_offline_hotwords_file() -> String {
    "./docs/hotword.md".to_string()
}

/// 热词加成权重 0.0-2.0 范围；0.0 关闭，1.0 中等，2.0 强烈
fn default_offline_hotwords_score() -> f32 {
    1.5
}

fn default_offline_num_threads() -> i32 {
    2
}

fn default_offline_provider() -> String {
    "cpu".to_string()
}

/// Zipformer CTC 模型 beam search 增益极小，greedy 既快又准。
fn default_offline_decoding() -> String {
    // Paraformer 在使用 hotwords_file 时 C-API 强制要求 modified_beam_search
    // （hotword bias 只能在 beam search 阶段加权，greedy 不支持）。
    // 即使没开热词，modified_beam_search 也会比 greedy 更准，代价是 ~10-30% 延迟。
    "modified_beam_search".to_string()
}

fn default_enable_offline_refine() -> bool {
    true
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
            offline_model_path: default_offline_model_path(),
            offline_tokens_path: default_offline_tokens_path(),
            offline_num_threads: default_offline_num_threads(),
            offline_provider: default_offline_provider(),
            offline_decoding: default_offline_decoding(),
            offline_hotwords_file: default_offline_hotwords_file(),
            offline_hotwords_score: default_offline_hotwords_score(),
            enable_offline_refine: default_enable_offline_refine(),
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
        assert_eq!(config.vad_threshold, 0.5);
        assert_eq!(config.enable_offline_refine, true);
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
    fn test_config_offline_fields_default() {
        let c = Config::default();
        // Paraformer-zh-2023-09-14 是 model.int8.onnx（int8 版本）
        assert!(c.offline_model_path.ends_with("model.int8.onnx"));
        assert!(c.offline_tokens_path.ends_with("tokens.txt"));
        assert!(c.offline_model_path.contains("paraformer-zh"));
        assert_eq!(c.offline_num_threads, 2);
        assert_eq!(c.offline_provider, "cpu");
        // 默认 modified_beam_search（Paraformer + 热词要求）
        assert_eq!(c.offline_decoding, "modified_beam_search");
        // 默认热词文件指向仓库自带的 docs/hotword.md
        assert!(c.offline_hotwords_file.ends_with("hotword.md"));
        assert_eq!(c.offline_hotwords_score, 1.5);
        assert!(c.enable_offline_refine);
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
        writeln!(temp_file, "offline_model_path = \"/custom/offline.onnx\"").unwrap();
        writeln!(temp_file, "offline_tokens_path = \"/custom/tokens.txt\"").unwrap();
        writeln!(temp_file, "offline_num_threads = 1").unwrap();
        writeln!(temp_file, "enable_offline_refine = false").unwrap();
        writeln!(temp_file, "hotkey = \"F4\"").unwrap();

        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.model_dir, "/custom/models");
        assert_eq!(config.vad_threshold, 0.7);
        assert_eq!(config.offline_model_path, "/custom/offline.onnx");
        assert_eq!(config.offline_tokens_path, "/custom/tokens.txt");
        assert_eq!(config.offline_num_threads, 1);
        assert_eq!(config.enable_offline_refine, false);
        assert_eq!(config.hotkey, "F4");
    }
}