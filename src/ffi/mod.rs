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
