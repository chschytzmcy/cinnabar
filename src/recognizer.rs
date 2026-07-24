use crate::ffi::{OnlineRecognizer, OnlineStream, VoiceActivityDetector};
use crate::resampler::LinearResampler;
use crate::vad::{VadConfig, drain_segments};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use crossbeam_channel::{bounded, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// GUI 模式下的识别引擎包装。同时持有 ASR recognizer、ten-vad VAD 和 cpal 音频流。
pub struct RecognizerEngine {
    recognizer: OnlineRecognizer,
    _stream: cpal::Stream,
    rx: Receiver<Vec<f32>>,
    running: Arc<AtomicBool>,
    resampler: Option<LinearResampler>,
    target_sample_rate: u32,
    vad: VoiceActivityDetector,
    vad_cfg: VadConfig,
    cumulative_segment_id: u64,
}

impl RecognizerEngine {
    pub fn new(
        model_dir: &std::path::Path,
        device_idx: Option<usize>,
        device_name: Option<String>,
        vad_config: VadConfig,
    ) -> Result<Self> {
        let recognizer = OnlineRecognizer::new(
            &model_dir.join("encoder.int8.onnx").to_string_lossy(),
            &model_dir.join("decoder.int8.onnx").to_string_lossy(),
            &model_dir.join("tokens.txt").to_string_lossy(),
            4,
        )?;

        let host = cpal::default_host();
        let device = if let Some(idx) = device_idx {
            host.input_devices()?
                .nth(idx)
                .context(format!("设备索引 {} 无效", idx))?
        } else if let Some(name) = &device_name {
            host.input_devices()?
                .find(|d| d.name().ok().as_ref() == Some(name))
                .context(format!("未找到设备名称: {}", name))?
        } else {
            host.default_input_device().context("未找到默认输入设备")?
        };

        let target_sample_rate = 16000;
        let supports_16khz = device
            .supported_input_configs()
            .ok()
            .and_then(|configs| {
                configs.filter(|c| c.channels() == 1).find(|c| {
                    let min = c.min_sample_rate().0;
                    let max = c.max_sample_rate().0;
                    target_sample_rate >= min && target_sample_rate <= max
                })
            })
            .is_some();

        let (config, use_resampler) = if supports_16khz {
            (
                cpal::StreamConfig {
                    channels: 1,
                    sample_rate: cpal::SampleRate(target_sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                false,
            )
        } else {
            let default_config = device.default_input_config()?;
            (
                cpal::StreamConfig {
                    channels: default_config.channels(),
                    sample_rate: default_config.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                },
                default_config.sample_rate().0 != target_sample_rate,
            )
        };

        let (tx, rx) = bounded::<Vec<f32>>(100);
        let channels = config.channels;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mono_data: Vec<f32> = if channels > 1 {
                    data.chunks(channels as usize)
                        .map(|chunk| chunk.iter().sum::<f32>() / (channels as f32).sqrt())
                        .collect()
                } else {
                    data.to_vec()
                };
                let _ = tx.try_send(mono_data);
            },
            |err| eprintln!("错误：{}", err),
            None,
        )?;

        let resampler = if use_resampler {
            Some(LinearResampler::new(
                config.sample_rate.0,
                target_sample_rate,
            ))
        } else {
            None
        };

        // ten-vad 由 CLI 模式同样的 VadConfig 路径构造；GUI 也用同一个 Config 字段。
        let vad = vad_config.build().context("创建 ten-vad 失败（GUI 模式）")?;

        Ok(Self {
            recognizer,
            _stream: stream,
            rx,
            running: Arc::new(AtomicBool::new(false)),
            resampler,
            target_sample_rate,
            vad,
            vad_cfg: vad_config,
            cumulative_segment_id: 0,
        })
    }

    pub fn start(&mut self) {
        self.running.store(true, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// 创建一段新的 ASR 流（GUI 端在 segment 结束时调用，绕开 sherpa-onnx 1.12.9 reset 崩溃）。
    pub fn create_stream(&self) -> OnlineStream {
        self.recognizer.create_stream()
    }

    /// 拉取一条音频并推进 ASR + ten-vad，返回这一轮的状态信号。
    ///
    /// 调用方按返回值三种情况分别处理：
    /// - `Idle`：还没准备好或没新音频
    /// - `Partial(text)`：ASR 出了新文本，刷新 UI
    /// - `SegmentEnded { text, .. }`：ten-vad 切 segment，提交并粘贴
    pub fn process(&mut self, stream: &mut OnlineStream) -> ProcessOutcome {
        if !self.running.load(Ordering::Relaxed) {
            return ProcessOutcome::Idle;
        }

        let samples = match self.rx.try_recv() {
            Ok(s) => s,
            Err(_) => return ProcessOutcome::Idle,
        };
        if samples.is_empty() {
            return ProcessOutcome::Idle;
        }

        // 重采样 → 16 kHz（VAD/ASR 双方都期望 16 kHz）
        let resampled = if let Some(ref mut r) = self.resampler {
            r.resample(&samples)
        } else {
            samples
        };
        if resampled.is_empty() {
            return ProcessOutcome::Idle;
        }

        // 1. ASR 吃数据
        stream.accept_waveform(self.target_sample_rate as i32, &resampled);
        // 2. VAD 也吃同样的数据
        self.vad.accept_waveform(&resampled);

        // 3. 触发解码
        while self.recognizer.is_ready(stream) {
            self.recognizer.decode(stream);
        }

        let result = self.recognizer.get_result(stream);
        let trimmed = result.trim();

        // 4. ten-vad 有没有切出新 segment？
        let segments = drain_segments(&self.vad);
        if !segments.is_empty() {
            // 用最后一条 segment 作为 segment_id 锚点；多 segment 罕见，丢点 id 关系不大。
            let _ = &segments;
            let segment_id = self.cumulative_segment_id;
            self.cumulative_segment_id = self.cumulative_segment_id.saturating_add(1);
            // 拿当前 ASR 的最终文本（与 main.rs CLI 路径同样的语义）
            return ProcessOutcome::SegmentEnded {
                text: trimmed.to_string(),
                segment_id,
            };
        }

        if !trimmed.is_empty() {
            return ProcessOutcome::Partial(trimmed.to_string());
        }

        ProcessOutcome::Idle
    }

    /// segment 结束后重建 ASR 流并清空 VAD 内部状态（GUI 端调用）。
    pub fn reset_vad(&mut self) {
        self.vad.reset();
    }

    /// 供 GUI 查询 VAD 配置（写日志/调试用）。
    #[allow(dead_code)]
    pub fn vad_config(&self) -> &VadConfig {
        &self.vad_cfg
    }
}

/// `RecognizerEngine::process` 的返回值。`SegmentEnded` 同时携带最终文本，
/// 让调用方（GUI）一次拿到 ASR 输出 + 提交信号，不必再二次轮询。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ProcessOutcome {
    /// 没新音频或识别器没准备好
    Idle,
    /// 流式识别的新部分文本
    Partial(String),
    /// ten-vad 切段完成，附带最终文本与本会话内 segment 自增 id
    SegmentEnded {
        text: String,
        segment_id: u64,
    },
}