//! `vad` —— ten-vad 的薄门面。
//!
//! 本模块不再做能量阈值或端点累积；只承担：
//! 1. 持有 `VadConfig`（含 CLI flag + config.toml + 内置默认 的合并结果）
//! 2. 构造 `ffi::VoiceActivityDetector`（配置走 ten-vad 槽位）
//! 3. 提供 `drain_segments` 让调用方一次性拿走所有就绪 segment
//!
//! 识别与端点的耦合由 main.rs / recognizer.rs 自行处理（每条 segment = 一次
//! ASR utterance commit + recognizer 流重建）。

use crate::ffi::{SpeechSegment, VoiceActivityDetector};
use anyhow::Result;

/// ten-vad 的运行时配置。所有字段都从 config.toml + CLI flag + 内置默认合并得来。
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// ONNX 模型绝对或相对路径（默认 `./models/ten-vad.onnx`）。
    pub model_path: String,
    /// 语音概率阈值 0-1（默认 0.5）。
    /// 与原能量阈值语义完全不同：现在是 ten-vad 模型输出的"是语音"概率。
    pub threshold: f32,
    /// 持续静音多久后切 segment（毫秒，默认 1200）。
    pub min_silence_ms: u32,
    /// 多短的语音才视为有效 segment（毫秒，默认 500）。
    pub min_speech_ms: u32,
    /// ten-vad 推理窗口大小（样本数，默认 256 = 16kHz 下 16ms）。
    /// 必须匹配 sample_rate，常见值 256 / 512 / 768。
    pub window_size: i32,
    /// 单个 segment 最长多少秒（默认 20.0），超过会被强制切断。
    pub max_speech_duration: f32,
    /// onnxruntime 推理线程数（默认 2）。
    pub num_threads: i32,
    /// 执行 provider（"cpu" / "cuda" / "coreml"，默认 "cpu"）。
    pub provider: String,
    /// 采样率 Hz（默认 16000）。
    pub sample_rate: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            model_path: "./models/ten-vad.onnx".into(),
            threshold: 0.5,
            min_silence_ms: 1200,
            min_speech_ms: 500,
            window_size: 256,
            max_speech_duration: 20.0,
            num_threads: 2,
            provider: "cpu".into(),
            sample_rate: 16000,
        }
    }
}

impl VadConfig {
    /// 构建一个 `VoiceActivityDetector` 实例。失败一般意味着模型路径不对、
    /// ONNX 文件损坏、或 window_size 与 sample_rate 不匹配。
    pub fn build(&self) -> Result<VoiceActivityDetector> {
        VoiceActivityDetector::new(
            &self.model_path,
            self.threshold,
            self.min_silence_ms as f32 / 1000.0,
            self.min_speech_ms as f32 / 1000.0,
            self.window_size,
            self.max_speech_duration,
            self.sample_rate as i32,
            self.num_threads,
            &self.provider,
            60.0, // buffer_size_in_seconds：足够缓存 ~60s 的音频避免长句丢失
        )
    }
}

/// 一次性拉取并弹出所有已就绪的 segment（按"先来先出"顺序）。
///
/// 调用前通常先 `vad.accept_waveform(...)`，然后让模型跑完；调本函数可以
/// 把"还没处理的 segment"批量取走。每取一个内部会 `front()+pop()`，samples
/// 已经被拷成 owned `Vec<f32>`，调用方拿到后即可放心持有。
///
/// 跳过 `samples.is_empty()` 的 segment（理论上不应出现，但前端做防御）。
pub fn drain_segments(vad: &VoiceActivityDetector) -> Vec<SpeechSegment> {
    let mut out = Vec::new();
    while !vad.is_empty() {
        let seg = vad.front();
        vad.pop();
        if !seg.samples.is_empty() {
            out.push(seg);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_sane() {
        let c = VadConfig::default();
        assert_eq!(c.sample_rate, 16000);
        assert!(c.threshold > 0.0 && c.threshold < 1.0);
        assert!(c.min_silence_ms >= 200);
        assert!(c.min_speech_ms >= 100);
        assert!(c.num_threads >= 1);
        assert!(c.window_size > 0);
        assert!(c.max_speech_duration > 0.0);
        assert_eq!(c.provider, "cpu");
    }
}