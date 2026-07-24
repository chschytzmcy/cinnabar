use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int};
use std::ptr;

#[repr(C)]
pub struct SherpaOnnxOnlineRecognizer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SherpaOnnxOnlineStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SherpaOnnxFeatureConfig {
    pub sample_rate: c_int,
    pub feature_dim: c_int,
}

#[repr(C)]
pub struct SherpaOnnxOnlineTransducerModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
    pub joiner: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOnlineParaformerModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOnlineZipformer2CtcModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOnlineNemoCtcModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOnlineModelConfig {
    pub transducer: SherpaOnnxOnlineTransducerModelConfig,
    pub paraformer: SherpaOnnxOnlineParaformerModelConfig,
    pub zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig,
    pub tokens: *const c_char,
    pub num_threads: c_int,
    pub provider: *const c_char,
    pub debug: c_int,
    pub model_type: *const c_char,
    pub modeling_unit: *const c_char,
    pub bpe_vocab: *const c_char,
    pub tokens_buf: *const c_char,
    pub tokens_buf_size: c_int,
    pub nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig,
}

#[repr(C)]
pub struct SherpaOnnxOnlineCtcFstDecoderConfig {
    pub graph: *const c_char,
    pub max_active: c_int,
}

#[repr(C)]
pub struct SherpaOnnxHomophoneReplacerConfig {
    pub dict_dir: *const c_char,
    pub lexicon: *const c_char,
    pub rule_fsts: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOnlineRecognizerConfig {
    pub feat_config: SherpaOnnxFeatureConfig,
    pub model_config: SherpaOnnxOnlineModelConfig,
    pub decoding_method: *const c_char,
    pub max_active_paths: c_int,
    pub enable_endpoint: c_int,
    pub rule1_min_trailing_silence: c_float,
    pub rule2_min_trailing_silence: c_float,
    pub rule3_min_utterance_length: c_float,
    pub hotwords_file: *const c_char,
    pub hotwords_score: c_float,
    pub ctc_fst_decoder_config: SherpaOnnxOnlineCtcFstDecoderConfig,
    pub rule_fsts: *const c_char,
    pub rule_fars: *const c_char,
    pub blank_penalty: c_float,
    pub hotwords_buf: *const c_char,
    pub hotwords_buf_size: c_int,
    pub hr: SherpaOnnxHomophoneReplacerConfig,
}

#[repr(C)]
pub struct SherpaOnnxOnlineRecognizerResult {
    pub text: *const c_char,
    pub tokens: *const c_char,
    pub tokens_arr: *const *const c_char,
    pub timestamps: *const c_float,
    pub count: c_int,
    pub json: *const c_char,
}

// --- VAD 配置结构（vendored c-api.h:832-888）---
// ten-vad 路径当前 vendored 库的唯一推荐选项；silero_vad 槽位保留但运行时置 null。
// 注：C 头文件全部使用 `*const` 而非 `*mut`，保持 API 对称。

#[repr(C)]
pub struct SherpaOnnxSileroVadModelConfig {
    pub model: *const c_char,
    pub threshold: c_float,
    pub min_silence_duration: c_float,
    pub min_speech_duration: c_float,
    pub window_size: c_int,
    pub max_speech_duration: c_float,
}

#[repr(C)]
pub struct SherpaOnnxTenVadModelConfig {
    pub model: *const c_char,
    pub threshold: c_float,
    pub min_silence_duration: c_float,
    pub min_speech_duration: c_float,
    pub window_size: c_int,
    pub max_speech_duration: c_float,
}

#[repr(C)]
pub struct SherpaOnnxVadModelConfig {
    pub silero_vad: SherpaOnnxSileroVadModelConfig,
    pub sample_rate: c_int,
    pub num_threads: c_int,
    pub provider: *const c_char,
    pub debug: c_int,
    pub ten_vad: SherpaOnnxTenVadModelConfig,
}

#[repr(C)]
pub struct SherpaOnnxSpeechSegment {
    pub start: c_int,
    pub samples: *mut c_float,
    pub n: c_int,
}

#[repr(C)]
pub struct SherpaOnnxVoiceActivityDetector {
    _private: [u8; 0],
}

// --- Offline recognizer 配置结构（vendored c-api.h:455-519）---
// 离线 API 没有 while-ready 循环：accept 一次 → decode 一次 → get_result。
// 句柄全部 `const *`（与 C 头文件一致），跟 online 现有 `*mut` 风格不一致
// 但 ABI 必须按 C 头来。

#[repr(C)]
pub struct SherpaOnnxOfflineLMConfig {
    pub model: *const c_char,
    pub scale: c_float,
}

// 占位：每个都按 c-api.h 实际字段写一遍（即使我们只用 Zipformer CTC），
// 因为 SherpaOnnxOfflineModelConfig 把它们全部 inline 进来，缺一个 layout 就错。
#[repr(C)]
pub struct SherpaOnnxOfflineTransducerModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
    pub joiner: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineParaformerModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineNemoEncDecCtcModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineWhisperModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
    pub language: *const c_char,
    pub task: *const c_char,
    pub tail_paddings: c_int,
}

#[repr(C)]
pub struct SherpaOnnxOfflineTdnnModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineSenseVoiceModelConfig {
    pub model: *const c_char,
    pub language: *const c_char,
    pub use_itn: c_int,
}

#[repr(C)]
pub struct SherpaOnnxOfflineMoonshineModelConfig {
    pub preprocessor: *const c_char,
    pub encoder: *const c_char,
    pub uncached_decoder: *const c_char,
    pub cached_decoder: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineFireRedAsrModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineDolphinModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineZipformerCtcModelConfig {
    pub model: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineCanaryModelConfig {
    pub encoder: *const c_char,
    pub decoder: *const c_char,
    pub src_lang: *const c_char,
    pub tgt_lang: *const c_char,
    pub use_pnc: c_int,
}

#[repr(C)]
pub struct SherpaOnnxOfflineModelConfig {
    pub transducer: SherpaOnnxOfflineTransducerModelConfig,
    pub paraformer: SherpaOnnxOfflineParaformerModelConfig,
    pub nemo_ctc: SherpaOnnxOfflineNemoEncDecCtcModelConfig,
    pub whisper: SherpaOnnxOfflineWhisperModelConfig,
    pub tdnn: SherpaOnnxOfflineTdnnModelConfig,
    pub tokens: *const c_char,
    pub num_threads: c_int,
    pub debug: c_int,
    pub provider: *const c_char,
    pub model_type: *const c_char,
    pub modeling_unit: *const c_char,
    pub bpe_vocab: *const c_char,
    pub telespeech_ctc: *const c_char,
    pub sense_voice: SherpaOnnxOfflineSenseVoiceModelConfig,
    pub moonshine: SherpaOnnxOfflineMoonshineModelConfig,
    pub fire_red_asr: SherpaOnnxOfflineFireRedAsrModelConfig,
    pub dolphin: SherpaOnnxOfflineDolphinModelConfig,
    pub zipformer_ctc: SherpaOnnxOfflineZipformerCtcModelConfig,
    pub canary: SherpaOnnxOfflineCanaryModelConfig,
}

#[repr(C)]
pub struct SherpaOnnxOfflineRecognizerConfig {
    pub feat_config: SherpaOnnxFeatureConfig,
    pub model_config: SherpaOnnxOfflineModelConfig,
    pub lm_config: SherpaOnnxOfflineLMConfig,
    pub decoding_method: *const c_char,
    pub max_active_paths: c_int,
    pub hotwords_file: *const c_char,
    pub hotwords_score: c_float,
    pub rule_fsts: *const c_char,
    pub rule_fars: *const c_char,
    pub blank_penalty: c_float,
    pub hr: SherpaOnnxHomophoneReplacerConfig,
}

// 字段顺序：text, timestamps, count, tokens, tokens_arr, json, lang, emotion, event
// 跟 online 不同（online 没有 lang/emotion/event），ABI 兼容必须严格按这个顺序
#[repr(C)]
pub struct SherpaOnnxOfflineRecognizerResult {
    pub text: *const c_char,
    pub timestamps: *const c_float,
    pub count: c_int,
    pub tokens: *const c_char,
    pub tokens_arr: *const *const c_char,
    pub json: *const c_char,
    pub lang: *const c_char,
    pub emotion: *const c_char,
    pub event: *const c_char,
}

#[repr(C)]
pub struct SherpaOnnxOfflineRecognizer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SherpaOnnxOfflineStream {
    _private: [u8; 0],
}

#[allow(dead_code)]
#[link(name = "sherpa-onnx-c-api")]
extern "C" {
    pub fn SherpaOnnxCreateOnlineRecognizer(
        config: *const SherpaOnnxOnlineRecognizerConfig,
    ) -> *mut SherpaOnnxOnlineRecognizer;

    pub fn SherpaOnnxDestroyOnlineRecognizer(recognizer: *mut SherpaOnnxOnlineRecognizer);

    pub fn SherpaOnnxCreateOnlineStream(
        recognizer: *const SherpaOnnxOnlineRecognizer,
    ) -> *mut SherpaOnnxOnlineStream;

    pub fn SherpaOnnxDestroyOnlineStream(stream: *mut SherpaOnnxOnlineStream);

    pub fn SherpaOnnxOnlineStreamAcceptWaveform(
        stream: *mut SherpaOnnxOnlineStream,
        sample_rate: c_int,
        samples: *const c_float,
        n: c_int,
    );

    pub fn SherpaOnnxIsOnlineStreamReady(
        recognizer: *mut SherpaOnnxOnlineRecognizer,
        stream: *mut SherpaOnnxOnlineStream,
    ) -> c_int;

    pub fn SherpaOnnxDecodeOnlineStream(
        recognizer: *mut SherpaOnnxOnlineRecognizer,
        stream: *mut SherpaOnnxOnlineStream,
    );

    pub fn SherpaOnnxGetOnlineStreamResult(
        recognizer: *const SherpaOnnxOnlineRecognizer,
        stream: *const SherpaOnnxOnlineStream,
    ) -> *const SherpaOnnxOnlineRecognizerResult;

    pub fn SherpaOnnxDestroyOnlineRecognizerResult(result: *const SherpaOnnxOnlineRecognizerResult);

    pub fn SherpaOnnxOnlineStreamIsEndpoint(stream: *mut SherpaOnnxOnlineStream) -> c_int;

    pub fn SherpaOnnxOnlineStreamReset(stream: *mut SherpaOnnxOnlineStream);

    // --- VoiceActivityDetector（ten-vad / silero-vad 通用）---
    // 11 个 C 函数 + 1 个 segment 析构函数。
    pub fn SherpaOnnxCreateVoiceActivityDetector(
        config: *const SherpaOnnxVadModelConfig,
        buffer_size_in_seconds: c_float,
    ) -> *const SherpaOnnxVoiceActivityDetector;

    pub fn SherpaOnnxDestroyVoiceActivityDetector(
        p: *const SherpaOnnxVoiceActivityDetector,
    );

    pub fn SherpaOnnxVoiceActivityDetectorAcceptWaveform(
        p: *const SherpaOnnxVoiceActivityDetector,
        samples: *const c_float,
        n: c_int,
    );

    pub fn SherpaOnnxVoiceActivityDetectorEmpty(
        p: *const SherpaOnnxVoiceActivityDetector,
    ) -> c_int;

    pub fn SherpaOnnxVoiceActivityDetectorDetected(
        p: *const SherpaOnnxVoiceActivityDetector,
    ) -> c_int;

    pub fn SherpaOnnxVoiceActivityDetectorPop(p: *const SherpaOnnxVoiceActivityDetector);

    pub fn SherpaOnnxVoiceActivityDetectorClear(p: *const SherpaOnnxVoiceActivityDetector);

    pub fn SherpaOnnxVoiceActivityDetectorFront(
        p: *const SherpaOnnxVoiceActivityDetector,
    ) -> *const SherpaOnnxSpeechSegment;

    pub fn SherpaOnnxDestroySpeechSegment(p: *const SherpaOnnxSpeechSegment);

    pub fn SherpaOnnxVoiceActivityDetectorReset(p: *const SherpaOnnxVoiceActivityDetector);

    pub fn SherpaOnnxVoiceActivityDetectorFlush(p: *const SherpaOnnxVoiceActivityDetector);

    // --- Offline Recognizer（非流式，一次性 accept → decode → get_result）---
    // 句柄按 C 头文件用 *const；没有 IsOfflineStreamReady（离线不需要）。
    pub fn SherpaOnnxCreateOfflineRecognizer(
        config: *const SherpaOnnxOfflineRecognizerConfig,
    ) -> *const SherpaOnnxOfflineRecognizer;

    pub fn SherpaOnnxDestroyOfflineRecognizer(
        p: *const SherpaOnnxOfflineRecognizer,
    );

    pub fn SherpaOnnxCreateOfflineStream(
        recognizer: *const SherpaOnnxOfflineRecognizer,
    ) -> *const SherpaOnnxOfflineStream;

    pub fn SherpaOnnxDestroyOfflineStream(stream: *const SherpaOnnxOfflineStream);

    /// 每条流只能调一次。
    pub fn SherpaOnnxAcceptWaveformOffline(
        stream: *const SherpaOnnxOfflineStream,
        sample_rate: c_int,
        samples: *const c_float,
        n: c_int,
    );

    pub fn SherpaOnnxDecodeOfflineStream(
        recognizer: *const SherpaOnnxOfflineRecognizer,
        stream: *const SherpaOnnxOfflineStream,
    );

    pub fn SherpaOnnxGetOfflineStreamResult(
        stream: *const SherpaOnnxOfflineStream,
    ) -> *const SherpaOnnxOfflineRecognizerResult;

    pub fn SherpaOnnxDestroyOfflineRecognizerResult(
        r: *const SherpaOnnxOfflineRecognizerResult,
    );
}

pub struct OnlineRecognizer {
    recognizer: *mut SherpaOnnxOnlineRecognizer,
    _encoder: CString,
    _decoder: CString,
    _tokens: CString,
    _provider: CString,
    _decoding: CString,
}

unsafe impl Send for OnlineRecognizer {}
unsafe impl Sync for OnlineRecognizer {}

pub struct OnlineStream {
    stream: *mut SherpaOnnxOnlineStream,
}

unsafe impl Send for OnlineStream {}
unsafe impl Sync for OnlineStream {}

impl OnlineRecognizer {
    pub fn new(
        encoder: &str,
        decoder: &str,
        tokens: &str,
        num_threads: i32,
    ) -> anyhow::Result<Self> {
        unsafe {
            let encoder_c = CString::new(encoder).unwrap();
            let decoder_c = CString::new(decoder).unwrap();
            let tokens_c = CString::new(tokens).unwrap();
            let provider_c = CString::new("cpu").unwrap();
            let decoding_c = CString::new("greedy_search").unwrap();

            let config = SherpaOnnxOnlineRecognizerConfig {
                feat_config: SherpaOnnxFeatureConfig {
                    sample_rate: 16000,
                    feature_dim: 80,
                },
                model_config: SherpaOnnxOnlineModelConfig {
                    transducer: SherpaOnnxOnlineTransducerModelConfig {
                        encoder: ptr::null(),
                        decoder: ptr::null(),
                        joiner: ptr::null(),
                    },
                    paraformer: SherpaOnnxOnlineParaformerModelConfig {
                        encoder: encoder_c.as_ptr(),
                        decoder: decoder_c.as_ptr(),
                    },
                    zipformer2_ctc: SherpaOnnxOnlineZipformer2CtcModelConfig { model: ptr::null() },
                    tokens: tokens_c.as_ptr(),
                    num_threads,
                    provider: provider_c.as_ptr(),
                    debug: 0,
                    model_type: ptr::null(),
                    modeling_unit: ptr::null(),
                    bpe_vocab: ptr::null(),
                    tokens_buf: ptr::null(),
                    tokens_buf_size: 0,
                    nemo_ctc: SherpaOnnxOnlineNemoCtcModelConfig { model: ptr::null() },
                },
                decoding_method: decoding_c.as_ptr(),
                max_active_paths: 4,
                enable_endpoint: 1,
                rule1_min_trailing_silence: 2.4,
                rule2_min_trailing_silence: 1.2,
                rule3_min_utterance_length: 0.0,
                hotwords_file: ptr::null(),
                hotwords_score: 0.0,
                ctc_fst_decoder_config: SherpaOnnxOnlineCtcFstDecoderConfig {
                    graph: ptr::null(),
                    max_active: 0,
                },
                rule_fsts: ptr::null(),
                rule_fars: ptr::null(),
                blank_penalty: 0.0,
                hotwords_buf: ptr::null(),
                hotwords_buf_size: 0,
                hr: SherpaOnnxHomophoneReplacerConfig {
                    dict_dir: ptr::null(),
                    lexicon: ptr::null(),
                    rule_fsts: ptr::null(),
                },
            };

            let recognizer = SherpaOnnxCreateOnlineRecognizer(&raw const config);
            if recognizer.is_null() {
                anyhow::bail!("创建识别器失败");
            }

            Ok(Self {
                recognizer,
                _encoder: encoder_c,
                _decoder: decoder_c,
                _tokens: tokens_c,
                _provider: provider_c,
                _decoding: decoding_c,
            })
        }
    }

    pub fn create_stream(&self) -> OnlineStream {
        unsafe {
            let stream = SherpaOnnxCreateOnlineStream(self.recognizer);
            OnlineStream { stream }
        }
    }

    pub fn is_ready(&self, stream: &OnlineStream) -> bool {
        unsafe { SherpaOnnxIsOnlineStreamReady(self.recognizer, stream.stream) != 0 }
    }

    pub fn decode(&self, stream: &mut OnlineStream) {
        unsafe {
            SherpaOnnxDecodeOnlineStream(self.recognizer, stream.stream);
        }
    }

    pub fn get_result(&self, stream: &OnlineStream) -> String {
        unsafe {
            let result = SherpaOnnxGetOnlineStreamResult(self.recognizer, stream.stream);
            if result.is_null() {
                return String::new();
            }
            // 关键：在 destroy result 之前先把 text 拷成 owned String。
            // sherpa-onnx 的 `text` 指针指向 result 内部 buffer；release + LTO 下
            // `to_string_lossy().to_string()` 容易被优化成延迟拷贝，destroy 之后再
            // 解引用 -> 段错误。这里把 to_string_lossy 的生命周期收敛在 result 存活
            // 的作用域内并用 into_owned() 强制落地，可彻底规避。
            let text = {
                let text_ptr = (*result).text;
                if text_ptr.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(text_ptr).to_string_lossy().into_owned()
                }
            };
            SherpaOnnxDestroyOnlineRecognizerResult(result);
            text
        }
    }

    /// 已弃用：使用 `vad::EndpointDetector` 替代
    ///
    /// sherpa-onnx 的 endpoint 检测在某些平台上存在崩溃问题。
    /// 推荐使用 `vad::EndpointDetector` 进行基于 VAD 和静音时长的 endpoint 检测。
    #[deprecated(since = "1.2.3", note = "使用 vad::EndpointDetector 替代")]
    #[allow(dead_code)]
    pub fn is_endpoint(&self, stream: &OnlineStream) -> bool {
        if stream.stream.is_null() {
            eprintln!("[WARNING] stream.stream is null in is_endpoint");
            return false;
        }
        // 禁用以避免崩溃，使用 vad::EndpointDetector 替代
        false
    }

    /// 重新创建流以绕开 sherpa-onnx 1.12.9 reset 路径下的状态损坏。
    /// 推荐在 endpoint 后调用 `create_stream` 重建流，而不是调用 `reset`。
    #[allow(dead_code)]
    pub fn reset(&self, stream: &mut OnlineStream) {
        unsafe {
            SherpaOnnxOnlineStreamReset(stream.stream);
        }
    }
}

impl OnlineStream {
    pub fn accept_waveform(&mut self, sample_rate: i32, samples: &[f32]) {
        unsafe {
            SherpaOnnxOnlineStreamAcceptWaveform(
                self.stream,
                sample_rate,
                samples.as_ptr(),
                samples.len() as c_int,
            );
        }
    }
}

impl Drop for OnlineRecognizer {
    fn drop(&mut self) {
        unsafe {
            SherpaOnnxDestroyOnlineRecognizer(self.recognizer);
        }
    }
}

impl Drop for OnlineStream {
    fn drop(&mut self) {
        unsafe {
            SherpaOnnxDestroyOnlineStream(self.stream);
        }
    }
}

// ============================================================================
// VoiceActivityDetector —— sherpa-onnx 1.12.9 的 VAD 包装
// ============================================================================
//
// 设计要点：
// - 持有 *const 句柄（与 C 头文件对称），以及两个 owned CString 保证 model path
//   与 provider 字符串的内存活得比 C 端调用更久（OnlineRecognizer 同款模式）。
// - front() 在 DestroySpeechSegment 之前把 samples 拷进 owned Vec<f32>，避免
//   LTO 下把拷贝推迟到 free 之后而踩悬空指针（参照 get_result 的修复模式）。
// - flush() 在 Ctrl+C 退出前调用，把已入队但还没切完的 segment 排出来。

#[derive(Clone, Debug)]
pub struct SpeechSegment {
    pub start: i32,
    pub samples: Vec<f32>,
}

pub struct VoiceActivityDetector {
    vad: *const SherpaOnnxVoiceActivityDetector,
    _model_path: CString,
    _provider: CString,
}

unsafe impl Send for VoiceActivityDetector {}
unsafe impl Sync for VoiceActivityDetector {}

impl VoiceActivityDetector {
    pub fn new(
        model_path: &str,
        threshold: f32,
        min_silence_duration: f32,
        min_speech_duration: f32,
        window_size: i32,
        max_speech_duration: f32,
        sample_rate: i32,
        num_threads: i32,
        provider: &str,
        buffer_size_in_seconds: f32,
    ) -> anyhow::Result<Self> {
        unsafe {
            let model_c = CString::new(model_path)
                .map_err(|e| anyhow::anyhow!("model_path 含 NUL: {}", e))?;
            let provider_c = CString::new(provider)
                .map_err(|e| anyhow::anyhow!("provider 含 NUL: {}", e))?;

            let config = SherpaOnnxVadModelConfig {
                // silero 槽位留空：本项目只用 ten-vad。
                silero_vad: SherpaOnnxSileroVadModelConfig {
                    model: ptr::null(),
                    threshold: 0.0,
                    min_silence_duration: 0.0,
                    min_speech_duration: 0.0,
                    window_size: 0,
                    max_speech_duration: 0.0,
                },
                sample_rate,
                num_threads,
                provider: provider_c.as_ptr(),
                debug: 0,
                ten_vad: SherpaOnnxTenVadModelConfig {
                    model: model_c.as_ptr(),
                    threshold,
                    min_silence_duration,
                    min_speech_duration,
                    window_size,
                    max_speech_duration,
                },
            };

            let vad = SherpaOnnxCreateVoiceActivityDetector(&raw const config, buffer_size_in_seconds);
            if vad.is_null() {
                anyhow::bail!(
                    "创建 ten-vad VoiceActivityDetector 失败（检查模型路径: {}）",
                    model_path
                );
            }

            Ok(Self {
                vad,
                _model_path: model_c,
                _provider: provider_c,
            })
        }
    }

    pub fn accept_waveform(&self, samples: &[f32]) {
        unsafe {
            SherpaOnnxVoiceActivityDetectorAcceptWaveform(
                self.vad,
                samples.as_ptr(),
                samples.len() as c_int,
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { SherpaOnnxVoiceActivityDetectorEmpty(self.vad) != 0 }
    }

    /// 当前是否有语音正在被检测到（per-frame "is currently speech"）。
    /// 等价于原 VadDetector::is_speech 的语义，但由 ten-vad 内部状态驱动。
    #[allow(dead_code)]
    pub fn is_speech_detected(&self) -> bool {
        unsafe { SherpaOnnxVoiceActivityDetectorDetected(self.vad) != 0 }
    }

    /// 取第一个 segment 并把 samples 拷成 owned Vec。
    /// 关键：c-api 的 Front() 在 C++ 侧 `new[]` 了 samples 数组，
    /// 我们必须在 DestroySpeechSegment 之前完成拷贝，否则 LTO 下会读到 free 后的内存。
    /// 调用方必须先用 `is_empty()` 检查，否则 seg 为 null 是 UB。
    pub fn front(&self) -> SpeechSegment {
        unsafe {
            let seg = SherpaOnnxVoiceActivityDetectorFront(self.vad);
            assert!(
                !seg.is_null(),
                "VoiceActivityDetector::front() 在 is_empty() 时是 UB"
            );
            let start = (*seg).start;
            let n = (*seg).n as usize;
            let mut samples = vec![0.0f32; n];
            if n > 0 {
                ptr::copy_nonoverlapping((*seg).samples, samples.as_mut_ptr(), n);
            }
            SherpaOnnxDestroySpeechSegment(seg);
            SpeechSegment { start, samples }
        }
    }

    pub fn pop(&self) {
        unsafe {
            SherpaOnnxVoiceActivityDetectorPop(self.vad);
        }
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        unsafe {
            SherpaOnnxVoiceActivityDetectorClear(self.vad);
        }
    }

    pub fn reset(&self) {
        unsafe {
            SherpaOnnxVoiceActivityDetectorReset(self.vad);
        }
    }

    pub fn flush(&self) {
        unsafe {
            SherpaOnnxVoiceActivityDetectorFlush(self.vad);
        }
    }
}

impl Drop for VoiceActivityDetector {
    fn drop(&mut self) {
        unsafe {
            SherpaOnnxDestroyVoiceActivityDetector(self.vad);
        }
    }
}

// ============================================================================
// OfflineRecognizer —— sherpa-onnx 1.12.9 的非流式 ASR 包装
// ============================================================================
//
// 与 OnlineRecognizer 的差异：
// - **没有 while-ready 循环**：accept 一次 → decode 一次 → get_result
// - **AcceptWaveformOffline 每条流只能调一次**（c-api.h:581 注释明示）
// - 句柄按 C 头文件用 *const（与现有 OnlineRecognizer 用 *mut 风格不同）
// - 用于精修（refine）：流式切段后，把整段音频另喂给 OfflineRecognizer，
//   拿更高质量（更低 CER）的最终文本

pub struct OfflineRecognizer {
    recognizer: *const SherpaOnnxOfflineRecognizer,
    _model_path: CString,
    _tokens_path: CString,
    _provider: CString,
    _decoding: CString,
}

unsafe impl Send for OfflineRecognizer {}
unsafe impl Sync for OfflineRecognizer {}

pub struct OfflineStream {
    stream: *const SherpaOnnxOfflineStream,
}

// OfflineStream 不需要 Send/Sync：每条流由单一线程持有、用完即 drop。
unsafe impl Send for OfflineStream {}
unsafe impl Sync for OfflineStream {}

impl OfflineRecognizer {
    /// 创建非流式 recognizer。当前实现只支持 Zipformer CTC（plan §1.1）。
    /// 模型路径是 `model.int8.onnx` 单文件；tokens.txt 是分词表。
    pub fn new(
        model_path: &str,
        tokens_path: &str,
        num_threads: i32,
        provider: &str,
        decoding_method: &str,
    ) -> anyhow::Result<Self> {
        unsafe {
            let model_c = CString::new(model_path)
                .map_err(|e| anyhow::anyhow!("model_path 含 NUL: {}", e))?;
            let tokens_c = CString::new(tokens_path)
                .map_err(|e| anyhow::anyhow!("tokens_path 含 NUL: {}", e))?;
            let provider_c = CString::new(provider)
                .map_err(|e| anyhow::anyhow!("provider 含 NUL: {}", e))?;
            let decoding_c = CString::new(decoding_method)
                .map_err(|e| anyhow::anyhow!("decoding_method 含 NUL: {}", e))?;

            let config = SherpaOnnxOfflineRecognizerConfig {
                feat_config: SherpaOnnxFeatureConfig {
                    sample_rate: 16000,
                    feature_dim: 80,
                },
                model_config: SherpaOnnxOfflineModelConfig {
                    // 占位全部置零 / null：本项目只用 zipformer_ctc
                    transducer: SherpaOnnxOfflineTransducerModelConfig {
                        encoder: ptr::null(),
                        decoder: ptr::null(),
                        joiner: ptr::null(),
                    },
                    paraformer: SherpaOnnxOfflineParaformerModelConfig { model: ptr::null() },
                    nemo_ctc: SherpaOnnxOfflineNemoEncDecCtcModelConfig { model: ptr::null() },
                    whisper: SherpaOnnxOfflineWhisperModelConfig {
                        encoder: ptr::null(),
                        decoder: ptr::null(),
                        language: ptr::null(),
                        task: ptr::null(),
                        tail_paddings: 0,
                    },
                    tdnn: SherpaOnnxOfflineTdnnModelConfig { model: ptr::null() },
                    tokens: tokens_c.as_ptr(),
                    num_threads,
                    debug: 0,
                    provider: provider_c.as_ptr(),
                    model_type: ptr::null(),
                    modeling_unit: ptr::null(),
                    bpe_vocab: ptr::null(),
                    telespeech_ctc: ptr::null(),
                    sense_voice: SherpaOnnxOfflineSenseVoiceModelConfig {
                        model: ptr::null(),
                        language: ptr::null(),
                        use_itn: 0,
                    },
                    moonshine: SherpaOnnxOfflineMoonshineModelConfig {
                        preprocessor: ptr::null(),
                        encoder: ptr::null(),
                        uncached_decoder: ptr::null(),
                        cached_decoder: ptr::null(),
                    },
                    fire_red_asr: SherpaOnnxOfflineFireRedAsrModelConfig {
                        encoder: ptr::null(),
                        decoder: ptr::null(),
                    },
                    dolphin: SherpaOnnxOfflineDolphinModelConfig { model: ptr::null() },
                    // ★ 真正在用的槽位
                    zipformer_ctc: SherpaOnnxOfflineZipformerCtcModelConfig {
                        model: model_c.as_ptr(),
                    },
                    canary: SherpaOnnxOfflineCanaryModelConfig {
                        encoder: ptr::null(),
                        decoder: ptr::null(),
                        src_lang: ptr::null(),
                        tgt_lang: ptr::null(),
                        use_pnc: 0,
                    },
                },
                lm_config: SherpaOnnxOfflineLMConfig {
                    model: ptr::null(),
                    scale: 0.0,
                },
                decoding_method: decoding_c.as_ptr(),
                max_active_paths: 4,
                hotwords_file: ptr::null(),
                hotwords_score: 0.0,
                rule_fsts: ptr::null(),
                rule_fars: ptr::null(),
                blank_penalty: 0.0,
                hr: SherpaOnnxHomophoneReplacerConfig {
                    dict_dir: ptr::null(),
                    lexicon: ptr::null(),
                    rule_fsts: ptr::null(),
                },
            };

            let recognizer = SherpaOnnxCreateOfflineRecognizer(&raw const config);
            if recognizer.is_null() {
                anyhow::bail!(
                    "创建非流式 recognizer 失败（检查模型路径: {} 与 tokens.txt: {}）",
                    model_path,
                    tokens_path
                );
            }

            Ok(Self {
                recognizer,
                _model_path: model_c,
                _tokens_path: tokens_c,
                _provider: provider_c,
                _decoding: decoding_c,
            })
        }
    }

    pub fn create_stream(&self) -> OfflineStream {
        unsafe {
            let stream = SherpaOnnxCreateOfflineStream(self.recognizer);
            OfflineStream { stream }
        }
    }
}

impl OfflineStream {
    /// 每条流只能调一次。重复调用是 UB（c-api.h:581 注释）。
    pub fn accept_waveform(&mut self, sample_rate: i32, samples: &[f32]) {
        unsafe {
            SherpaOnnxAcceptWaveformOffline(
                self.stream,
                sample_rate,
                samples.as_ptr(),
                samples.len() as c_int,
            );
        }
    }

    pub fn decode(&mut self, recognizer: &OfflineRecognizer) {
        unsafe {
            SherpaOnnxDecodeOfflineStream(recognizer.recognizer, self.stream);
        }
    }

    /// 取最终文本。必须在 decode() 之后调用。
    /// 关键：把 text 拷成 owned String 再 DestroyOfflineRecognizerResult，
    /// 否则 LTO 下会读到 free 后的内存（参照 OnlineRecognizer::get_result）。
    pub fn get_result(&self) -> String {
        unsafe {
            let result = SherpaOnnxGetOfflineStreamResult(self.stream);
            if result.is_null() {
                return String::new();
            }
            let text = {
                let text_ptr = (*result).text;
                if text_ptr.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(text_ptr).to_string_lossy().into_owned()
                }
            };
            SherpaOnnxDestroyOfflineRecognizerResult(result);
            text
        }
    }
}

impl Drop for OfflineRecognizer {
    fn drop(&mut self) {
        unsafe {
            SherpaOnnxDestroyOfflineRecognizer(self.recognizer);
        }
    }
}

impl Drop for OfflineStream {
    fn drop(&mut self) {
        unsafe {
            SherpaOnnxDestroyOfflineStream(self.stream);
        }
    }
}
