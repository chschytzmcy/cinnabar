mod config;
mod display;
mod ffi;
mod gui;
mod injector;
mod itn;
mod logger;
mod recognizer;
mod refine_score;
mod resampler;
mod vad;
mod wayland;
mod wav_source;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, RecvTimeoutError};
use ffi::{OfflineRecognizer, OnlineRecognizer};
use logger::{SegmentCommitInfo, SessionEndReason, SessionLog, SessionStartInfo};
use resampler::LinearResampler;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use vad::{VadConfig, drain_segments};

#[derive(Parser, Debug)]
#[command(name = "cinnabar")]
#[command(about = "轻量级、离线优先的 Linux 流式语音转文字工具")]
pub struct Args {
    /// 运行模式：cli 或 gui
    #[arg(short, long, default_value = "cli")]
    mode: String,

    #[arg(short = 'M', long, default_value = "./models")]
    model_dir: PathBuf,

    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    list_devices: bool,

    #[arg(short, long)]
    device: Option<usize>,

    #[arg(long)]
    device_name: Option<String>,

    /// 从本地 wav 文件输入而非麦克风。
    /// 要求 PCM s16le / 16 kHz / 单声道（10 秒 = 160 000 样本）。
    /// 文件读完即走 vad.flush() + drain + 正常退出（session_end 标 CleanExit）。
    #[arg(long, value_name = "PATH")]
    input_file: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,

    /// 流式识别日志输出路径（JSONL）。默认 $XDG_DATA_HOME/cinnabar/sessions/。
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// 流式 partial 的最短输出间隔（毫秒）。
    /// 值越小越跟手，但 partial 越多；150ms 是经验上比较舒服的折中。
    /// 仅约束 partial 打印的节奏，不影响 endpoint 最终结果的立即输出。
    #[arg(long, default_value_t = 150)]
    debounce_ms: u64,

    // --- ten-vad CLI 覆盖（优先级最高） ---
    /// ten-vad ONNX 模型路径；覆盖 config.toml 的 vad_model_path。
    #[arg(long)]
    vad_model: Option<String>,

    /// ten-vad 语音概率阈值 0.0-1.0；覆盖 config.toml 的 vad_threshold。
    #[arg(long)]
    vad_threshold: Option<f32>,

    /// ten-vad 切 segment 所需的持续静音时长（毫秒）；覆盖 config.toml。
    #[arg(long)]
    vad_min_silence_ms: Option<u32>,

    /// ten-vad 视语音有效所需的最短时长（毫秒）；覆盖 config.toml。
    #[arg(long)]
    vad_min_speech_ms: Option<u32>,

    // --- 非流式 ASR refine ---
    /// 禁用离线 ASR 精修（退回纯流式，节省 ~300MB 内存）。
    #[arg(long)]
    no_offline_refine: bool,

    /// 非流式 Zipformer CTC 模型路径；覆盖 config.toml。
    #[arg(long)]
    offline_model: Option<String>,

    /// 非流式 tokens.txt 路径；覆盖 config.toml。
    #[arg(long)]
    offline_tokens: Option<String>,

    /// 非流式推理线程数；覆盖 config.toml。
    #[arg(long)]
    offline_threads: Option<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 加载 config.toml（CLI 模式此前不读 config，本轮修上）。
    // 优先级：--vad-* CLI flag > config.toml > VadConfig::default()。
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from("./config.toml"));
    let file_config = Config::load(&config_path)
        .with_context(|| format!("加载配置文件失败: {}", config_path.display()))?;

    // 合并 VadConfig：CLI > 文件 > 默认
    let mut vad_cfg = VadConfig {
        model_path: file_config.vad_model_path.clone(),
        threshold: file_config.vad_threshold,
        min_silence_ms: file_config.vad_min_silence_ms,
        min_speech_ms: file_config.vad_min_speech_ms,
        window_size: file_config.vad_window_size,
        max_speech_duration: file_config.vad_max_speech_duration,
        num_threads: file_config.vad_num_threads,
        provider: file_config.vad_provider.clone(),
        sample_rate: 16000,
    };
    if let Some(p) = &args.vad_model {
        vad_cfg.model_path = p.clone();
    }
    if let Some(t) = args.vad_threshold {
        vad_cfg.threshold = t;
    }
    if let Some(s) = args.vad_min_silence_ms {
        vad_cfg.min_silence_ms = s;
    }
    if let Some(s) = args.vad_min_speech_ms {
        vad_cfg.min_speech_ms = s;
    }

    // 合并 OfflineRecognizer 配置：CLI > 文件 > 默认
    let offline_model_path = args
        .offline_model
        .clone()
        .unwrap_or_else(|| file_config.offline_model_path.clone());
    let offline_tokens_path = args
        .offline_tokens
        .clone()
        .unwrap_or_else(|| file_config.offline_tokens_path.clone());
    let offline_num_threads = args
        .offline_threads
        .unwrap_or(file_config.offline_num_threads);
    let offline_provider = file_config.offline_provider.clone();
    let offline_decoding = file_config.offline_decoding.clone();
    let enable_offline_refine = !args.no_offline_refine && file_config.enable_offline_refine;

    // 流式日志：尽早打开，CLI 列表设备阶段也记 session_start 以便事后追溯
    let mut session_log = SessionLog::open(args.log_file.as_deref())
        .context("打开会话日志失败")?;
    eprintln!("📝 会话日志: {}", session_log.path().display());

    // 模式切换
    match args.mode.as_str() {
        "gui" => return gui::run_gui_mode(&args),
        "cli" => {} // 继续执行 CLI 模式
        _ => anyhow::bail!("无效的模式。使用 'cli' 或 'gui'"),
    }

    // CLI 模式
    let host = cpal::default_host();

    if args.list_devices && args.input_file.is_some() {
        anyhow::bail!("--list-devices 与 --input-file 互斥");
    }

    if args.list_devices {
        println!("可用的音频输入设备：\n");
        for (idx, device) in host.input_devices()?.enumerate() {
            let name = device.name().unwrap_or_else(|_| "未知设备".to_string());
            let config = device.default_input_config();
            match config {
                Ok(cfg) => println!(
                    "  [{}] {} - {} Hz, {} 声道",
                    idx,
                    name,
                    cfg.sample_rate().0,
                    cfg.channels()
                ),
                Err(_) => println!("  [{}] {} - 无法获取配置", idx, name),
            }
        }
        return Ok(());
    }

    if !args.model_dir.exists() {
        anyhow::bail!("未找到模型目录：{}", args.model_dir.display());
    }

    // 流式 ASR 模型：Zipformer transducer（127MB, 中英双语 + 内置标点）
let stream_subdir = "sherpa-onnx-x-asr-960ms-streaming-zipformer-transducer-zh-en-punct-int8-2026-06-05";
let recognizer = OnlineRecognizer::new(
    &args
        .model_dir
        .join(stream_subdir)
        .join("encoder.int8.onnx")
        .to_string_lossy(),
    &args
        .model_dir
        .join(stream_subdir)
        .join("decoder.onnx")
        .to_string_lossy(),
    &args
        .model_dir
        .join(stream_subdir)
        .join("joiner.int8.onnx")
        .to_string_lossy(),
    &args
        .model_dir
        .join(stream_subdir)
        .join("tokens.txt")
        .to_string_lossy(),
    4,
)?;

    let mut stream = recognizer.create_stream();

    // 共享通道与 ctrlc：两条路径（mic / wav）都把样本喂到同一 tx。
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::Relaxed);
    })?;

    let (tx, rx) = bounded::<Vec<f32>>(100);
    let _verbose = args.verbose;

    // ── 音频源分支：本地 wav 文件 vs 麦克风 ──
    // 两条路径都把 f32 mono 16 kHz 样本送到同一个 tx；下游主循环不感知来源。
    let target_sample_rate: u32 = 16000;
    let actual_sample_rate: u32;
    let mut resampler: Option<LinearResampler> = None;
    let mut session_end_reason = SessionEndReason::CtrlC;
    let mut wav_handle: Option<JoinHandle<Result<u64>>> = None;
    let session_device: String;
    let mut session_channels: u16 = 1;
    let mut session_resampled = false;
    // cpal::Stream 必须在 main 全程保持存活；Drop 时回调会自动停止。
    let mut _audio_stream_keepalive: Option<cpal::Stream> = None;

    if let Some(path) = args.input_file.clone() {
        // ── 文件输入 ──
        if !path.exists() {
            anyhow::bail!("--input-file 指定的文件不存在: {}", path.display());
        }
        println!("📁 输入文件: {}", path.display());
        session_device = format!("file://{}", path.display());
        actual_sample_rate = target_sample_rate;
        wav_handle = Some(wav_source::spawn(
            &path,
            tx,
            wav_source::DEFAULT_CHUNK_SAMPLES,
        ));
        session_end_reason = SessionEndReason::CleanExit;
    } else {
        // ── 麦克风输入（原有 cpal 路径） ──
        let host = cpal::default_host();

        let device = if let Some(idx) = args.device {
            host.input_devices()?
                .nth(idx)
                .context(format!("设备索引 {} 无效", idx))?
        } else if let Some(name) = &args.device_name {
            host.input_devices()?
                .find(|d| d.name().ok().as_ref() == Some(name))
                .context(format!("未找到设备名称: {}", name))?
        } else {
            host.default_input_device().context("未找到默认输入设备")?
        };

        let device_name = device.name().unwrap_or_else(|_| "未知设备".to_string());
        println!("🎤 使用设备: {}", device_name);
        session_device = device_name;

        // 检查设备是否支持 16kHz 单声道配置
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
            println!("🔧 使用配置: 16000 Hz, 1 声道");
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
            let sample_rate = default_config.sample_rate().0;
            println!(
                "⚠️  16kHz 不支持，使用默认配置: {} Hz, {} 声道（将启用重采样）",
                sample_rate,
                default_config.channels()
            );
            (
                cpal::StreamConfig {
                    channels: default_config.channels(),
                    sample_rate: default_config.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                },
                sample_rate != target_sample_rate,
            )
        };

        actual_sample_rate = config.sample_rate.0;
        session_channels = config.channels;
        session_resampled = use_resampler;
        resampler = if use_resampler {
            Some(LinearResampler::new(actual_sample_rate, target_sample_rate))
        } else {
            None
        };

        let channels = config.channels;
        let audio_stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mono_data: Vec<f32> = if channels > 1 {
                    data.chunks(channels as usize)
                        .map(|chunk| {
                            let sum: f32 = chunk.iter().sum();
                            // 使用 sqrt(channels) 作为除数，避免音量过小
                            sum / (channels as f32).sqrt()
                        })
                        .collect()
                } else {
                    data.to_vec()
                };
                let _ = tx.try_send(mono_data);
            },
            |err| eprintln!("错误：{}", err),
            None,
        )?;

        audio_stream.play()?;
        println!("开始监听... 按 Ctrl+C 停止");
        _audio_stream_keepalive = Some(audio_stream);
    }

    session_log.session_start(&SessionStartInfo {
        mode: "cli".to_string(),
        model_dir: args.model_dir.display().to_string(),
        device: session_device.clone(),
        sample_rate: actual_sample_rate,
        channels: session_channels,
        resampled: session_resampled,
        vad_model_path: vad_cfg.model_path.clone(),
        vad_threshold: vad_cfg.threshold,
        min_silence_ms: vad_cfg.min_silence_ms,
        min_speech_ms: vad_cfg.min_speech_ms,
    });

    // ten-vad：神经网络 VAD，由 sherpa-onnx C-API 直接驱动。
    // 失败一般意味着模型路径错、ONNX 损坏、或 window_size 与 sample_rate 不匹配。
    let vad = vad_cfg
        .build()
        .context("创建 ten-vad VoiceActivityDetector 失败（检查 --vad-model 路径）")?;

    // 非流式 ASR（Paraformer zh attention decoder）：用于切段后的精修。
    // 加载失败 / 配置关闭时为 None，主循环里跳过 refine 走纯流式。
    // 空字符串的 hotwords_file 表示不启用热词。
    let offline: Option<OfflineRecognizer> = if enable_offline_refine {
        let hw_file = if file_config.offline_hotwords_file.is_empty() {
            None
        } else {
            Some(file_config.offline_hotwords_file.as_str())
        };
        match OfflineRecognizer::new(
            &offline_model_path,
            &offline_tokens_path,
            offline_num_threads,
            &offline_provider,
            &offline_decoding,
            hw_file,
            file_config.offline_hotwords_score,
        ) {
            Ok(r) => {
                let hw_info = if hw_file.is_some() {
                    format!(
                        "，热词加成 {:.1}",
                        file_config.offline_hotwords_score
                    )
                } else {
                    String::new()
                };
                eprintln!(
                    "🔧 非流式 ASR 已加载（Paraformer，refine 开启；~+200ms 延迟{}）",
                    hw_info
                );
                Some(r)
            }
            Err(e) => {
                eprintln!(
                    "⚠️  非流式 ASR 加载失败，退回纯流式：{}",
                    e
                );
                None
            }
        }
    } else {
        eprintln!("ℹ️  非流式 refine 已禁用（--no-offline-refine 或 config.toml）");
        None
    };

    // 流式输出状态：
    // - last_result：ASR 最近一次识别的文本
    // - last_printed：屏幕上/管道里最后一次出现的文本（避免重复输出）
    // - last_print_time：节流上次打印的时间戳
    let mut last_result = String::new();
    let mut last_printed = String::new();
    let mut last_committed_final = String::new();
    let mut last_print_time = std::time::Instant::now();
    let mut partial_seq: u64 = 0;
    let mut samples_in_session: u64 = 0;
    let mut segment_id: u64 = 0;

    // TTY 检测：终端下用 \r 就地更新，piped 下保持行式输出（更友好）。
    let stdout_is_tty = std::io::stdout().is_terminal();
    let debounce = std::time::Duration::from_millis(args.debounce_ms);

    while running.load(Ordering::Relaxed) {
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(samples) => {
                if samples.is_empty() {
                    continue;
                }

                let samples_16k = if let Some(ref mut r) = resampler {
                    r.resample(&samples)
                } else {
                    samples
                };

                // 检查重采样后的数据是否为空
                if samples_16k.is_empty() {
                    continue;
                }

                stream.accept_waveform(target_sample_rate as i32, &samples_16k);
                samples_in_session = samples_in_session.saturating_add(samples_16k.len() as u64);

                // VAD 也吃同样的 16 kHz 样本，与 ASR 并行推进；VAD 自身决定何时切段。
                // 先于 partial 打印之前调用，避免 partial 还没刷出来就被 final 覆盖掉。
                vad.accept_waveform(&samples_16k);

                // ten-vad 的 flush() 强制吐出 pending segment。
                // 长静音时（用户停顿 > min_silence_ms），ten-vad 内部状态机不会自然 emit
                // 段，必须手动 flush 触发。否则用户停顿时 VAD 不会自动 commit final，
                // 必须 Ctrl+C 才能 commit（退回到 last_printed 兜底逻辑，丢失准确率）。
                // 调用条件：当前帧是静音（即 is_speech_detected == false），避免频繁调。
                if !vad.is_speech_detected() {
                    vad.flush();
                }

                while recognizer.is_ready(&stream) {
                    recognizer.decode(&mut stream);
                }
                let result = recognizer.get_result(&stream);
                let trimmed = result.trim();

                if !trimmed.is_empty() && trimmed != last_result {
                    last_result = trimmed.to_string();
                    partial_seq = partial_seq.saturating_add(1);
                    // 文本变更即落日志（printed=false），便于事后回放 ASR 模型的贪心解码轨迹；
                    // 真正打印到屏幕时再补一条 printed=true，便于区分"模型识别的轨迹"和
                    // "用户看到的轨迹"。
                    session_log.partial(partial_seq, trimmed, samples_in_session, false);
                }

                // 节流 partial 打印：距上次打印至少 debounce_ms，且内容有变化。
                // 与原来的 "500ms 无变化" 不同，这里只看打印节奏，不等 ASR 文本稳定，
                // 所以跟手感大幅提升。TTY 下走就地覆盖（\r + ANSI clear-line），
                // piped 下走普通 println，避免污染下游管道消费者。
                if !last_result.is_empty()
                    && last_result != last_printed
                    && last_print_time.elapsed() >= debounce
                {
                    display::print_partial(&last_result, stdout_is_tty);
                    last_printed = last_result.clone();
                    last_print_time = std::time::Instant::now();
                    // 补一条"用户可见"的日志，printed=true。
                    session_log.partial(partial_seq, &last_printed, samples_in_session, true);
                }

                let segments = drain_segments(&vad);
                if !segments.is_empty() {
                    if args.verbose {
                        eprintln!(
                            "[DEBUG] ten-vad 切出 {} 个 segment（id={}, samples={}, duration={}ms）",
                            segments.len(),
                            segment_id,
                            segments.iter().map(|s| s.samples.len()).sum::<usize>(),
                            segments
                                .iter()
                                .map(|s| (s.samples.len() as u64 * 1000)
                                    / (vad_cfg.sample_rate as u64).max(1))
                                .sum::<u64>()
                        );
                    }
                    for seg in &segments {
                        let final_result = recognizer.get_result(&stream);
                        let trimmed_final = final_result.trim();
                        let duration_ms = (seg.samples.len() as u64 * 1000)
                            / (vad_cfg.sample_rate as u64).max(1);
                        session_log.endpoint_detected(&SegmentCommitInfo {
                            segment_id,
                            start_sample: seg.start,
                            samples: seg.samples.len() as u32,
                            duration_ms,
                        });
                        if !trimmed_final.is_empty() {
                            // TTY 下覆盖前一行 partial（如果还在），并以 \n 结束本行，
                            // 让下一句 partial 从新行开始。piped 下保留 \n✅ 前缀风格。
                            display::print_final(trimmed_final, stdout_is_tty);
                            last_printed = trimmed_final.to_string();
                            last_committed_final = trimmed_final.to_string();
                            session_log.final_result(trimmed_final);

                            // 非流式 ASR 精修：~200ms 内出结果，按 Jaccard 阈值决策。
                            // - 高一致：直接覆盖屏幕
                            // - 中等一致：覆盖但打 warn（精修可疑）
                            // - 低一致：**不覆盖**，保留流式结果（精修可能错）
                            // 紧跟 print_final 立即调用，期间不能有其它 stdout 输出，
                            // 否则 \x1b[1A 会覆盖错行。
                            if let Some(ref off) = offline {
                                match refine_segment(off, &seg.samples, target_sample_rate as i32) {
                                    Ok(refined) if !refined.is_empty() => {
                                        let score = refine_score::combined_score(trimmed_final, &refined);
                                        let decision = refine_score::refine_decision(score);
                                        match decision {
                                            "override" => {
                                                if refined != trimmed_final {
                                                    display::print_final_replace(&refined, stdout_is_tty);
                                                    last_printed = refined.clone();
                                                    last_committed_final = refined.clone();
                                                }
                                            }
                                            "override_warn" => {
                                                if args.verbose {
                                                    eprintln!(
                                                        "[INFO] refine 综合分 {:.2} 偏低，覆盖屏幕但记 warn",
                                                        score
                                                    );
                                                }
                                                if refined != trimmed_final {
                                                    display::print_final_replace(&refined, stdout_is_tty);
                                                    last_printed = refined.clone();
                                                    last_committed_final = refined.clone();
                                                }
                                            }
                                            "rejected" => {
                                                if args.verbose {
                                                    eprintln!(
                                                        "[INFO] refine 综合分 {:.2} 低于阈值，保留流式结果",
                                                        score
                                                    );
                                                }
                                                // 不覆盖屏幕，让用户看到流式版本
                                            }
                                            _ => unreachable!(),
                                        }
                                        session_log.refine(trimmed_final, &refined, score, decision);
                                    }
                                    Ok(_) => {
                                        // refine 返回空字符串：罕见，记 warn 但不阻塞
                                        if args.verbose {
                                            eprintln!("[DEBUG] 非流式 refine 返回空文本，跳过覆盖");
                                        }
                                    }
                                    Err(e) => {
                                        if args.verbose {
                                            eprintln!("[DEBUG] 非流式 refine 失败：{}", e);
                                        }
                                    }
                                }
                            }
                        }
                        segment_id = segment_id.saturating_add(1);
                    }
                    // 绕过 sherpa-onnx 1.12.9 OnlineStreamReset 路径下的状态损坏：
                    // 直接销毁当前 OnlineStream，再用 recognizer 重新创建一个。
                    // ten-vad 的 reset() 是独立的 C++ 对象调用，与 recognizer 流无关。
                    stream = recognizer.create_stream();
                    vad.reset();
                    // 重建流后，partial 也必须重新开始计数，否则 debounce 会错乱。
                    last_result.clear();
                    last_print_time = std::time::Instant::now();
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                // 音频源已退出（文件模式：feeder 读完；或消费端抢关）。
                // 把 running 置 false 让主循环退出，走 flush + drain + session_end。
                running.store(false, Ordering::Relaxed);
            }
            Err(RecvTimeoutError::Timeout) => {
                // 100ms 内没新数据：正常超时，继续轮询。
                continue;
            }
        }
    }

    // 文件模式下先 join feeder，确认所有 chunk 都已入队 + 拿到读取错误（若有）。
    // wav_handle 在主循环内已读完 + drop tx，主循环因此收到 Disconnected 退出。
    // 但 join 仍能让我们拿到 feeder 的返回值（Ok(总样本数) / Err(读取失败)）。
    if let Some(handle) = wav_handle.take() {
        match handle.join() {
            Ok(Ok(total)) => {
                eprintln!(
                    "[INFO] wav 已读完 {} 个样本（={}ms @ 16 kHz）",
                    total,
                    total * 1000 / 16_000
                );
            }
            Ok(Err(e)) => {
                eprintln!("[WARN] wav 读取失败: {:#}", e);
            }
            Err(_) => {
                eprintln!("[WARN] wav feeder 线程 panic");
            }
        }
    }

    // Ctrl+C 退出前 flush()，把已经推进去但还没切完的样本里残余 segment 排出来，
    // 避免最后一段说到一半被丢弃。
    vad.flush();
    let drained = drain_segments(&vad);
    let drained_count = drained.len();
    for seg in drained {
        let final_result = recognizer.get_result(&stream);
        let trimmed_final = final_result.trim();
        let duration_ms = (seg.samples.len() as u64 * 1000)
            / (vad_cfg.sample_rate as u64).max(1);
        session_log.endpoint_detected(&SegmentCommitInfo {
            segment_id,
            start_sample: seg.start,
            samples: seg.samples.len() as u32,
            duration_ms,
        });
        if !trimmed_final.is_empty() {
            display::print_final(trimmed_final, stdout_is_tty);
            // 循环已退出，不再打印 partial，last_printed 不再被读取。
            last_printed = trimmed_final.to_string();
            last_committed_final = trimmed_final.to_string();
            session_log.final_result(trimmed_final);

            // 退出路径上的 refine：跟行内 commit 同样的逻辑
            if let Some(ref off) = offline {
                if let Ok(refined) =
                    refine_segment(off, &seg.samples, target_sample_rate as i32)
                {
                    if !refined.is_empty() {
                        let score = refine_score::combined_score(trimmed_final, &refined);
                        let decision = refine_score::refine_decision(score);
                        match decision {
                            "override" | "override_warn" => {
                                if refined != trimmed_final {
                                    display::print_final_replace(&refined, stdout_is_tty);
                                    last_printed = refined.clone();
                                    last_committed_final = refined.clone();
                                }
                            }
                            "rejected" => {
                                if args.verbose {
                                    eprintln!(
                                        "[INFO] refine 综合分 {:.2} 低于阈值，保留流式结果",
                                        score
                                    );
                                }
                            }
                            _ => unreachable!(),
                        }
                        session_log.refine(trimmed_final, &refined, score, decision);
                    }
                }
            }
        }
        segment_id = segment_id.saturating_add(1);
    }

    // 兜底：vad 没切出任何 segment（min_silence 阈值未达到、用户 Ctrl+C 太急），
    // 但屏幕 / partial 日志里已经有最后一段文本（last_printed）。
    // 这种情况下 partial 事件被记录了，但 final 事件缺失 —— 用 last_printed 兜一条 final，
    // 原则：用户主动 Ctrl+C，模型识别了多少就 commit 多少，不丢。
    //
    // 触发条件：
    // - drained_count == 0（vad 队列空）
    // - last_printed != last_committed_final（有未提交的已打印文本）
    if drained_count == 0 && !last_printed.is_empty() && last_printed != last_committed_final {
        session_log.final_result(&last_printed);
        eprintln!(
            "[INFO] Ctrl+C 时 vad 未切段，兜底提交 last_printed（{} 字符）",
            last_printed.chars().count()
        );
    }

    session_log.session_end(session_end_reason);
    Ok(())
}

/// 把一段音频喂给非流式 recognizer 拿精修文本。
///
/// 创建一条 OfflineStream、accept 一次、decode 一次、读 result。
/// OfflineStream 在本函数返回前 drop，完整生命周期在栈上。
/// 失败（accept/decode/get_result 任一阶段）返回 anyhow::Error，调用方跳过 refine。
fn refine_segment(
    offline: &OfflineRecognizer,
    samples: &[f32],
    sample_rate: i32,
) -> Result<String> {
    let mut stream = offline.create_stream();
    stream.accept_waveform(sample_rate, samples);
    stream.decode(offline);
    let text = stream.get_result();
    Ok(text)
}

#[cfg(test)]
mod tests {
    // refine_score 的 13 个测试已迁到 src/refine_score.rs
    // 这里的测试只针对 main.rs 主循环集成
}
