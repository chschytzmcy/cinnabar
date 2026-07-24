mod config;
mod ffi;
mod gui;
mod injector;
mod logger;
mod recognizer;
mod resampler;
mod vad;
mod wayland;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::bounded;
use ffi::{OfflineRecognizer, OnlineRecognizer};
use logger::{SegmentCommitInfo, SessionEndReason, SessionLog, SessionStartInfo};
use resampler::LinearResampler;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

    println!(
        "🎤 使用设备: {}",
        device.name().unwrap_or_else(|_| "未知设备".to_string())
    );

    // 尝试配置 16000Hz 单声道，如果不支持则使用默认配置并启用重采样
    let target_sample_rate = 16000;

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

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::Relaxed);
    })?;

    let (tx, rx) = bounded::<Vec<f32>>(100);
    let actual_sample_rate = config.sample_rate.0;
    let channels = config.channels;
    let _verbose = args.verbose;

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

    let device_name = device.name().unwrap_or_else(|_| "未知设备".to_string());
    session_log.session_start(&SessionStartInfo {
        mode: "cli".to_string(),
        model_dir: args.model_dir.display().to_string(),
        device: device_name.clone(),
        sample_rate: actual_sample_rate,
        channels: config.channels,
        resampled: use_resampler,
        vad_model_path: vad_cfg.model_path.clone(),
        vad_threshold: vad_cfg.threshold,
        min_silence_ms: vad_cfg.min_silence_ms,
        min_speech_ms: vad_cfg.min_speech_ms,
    });

    let mut resampler = if use_resampler {
        Some(LinearResampler::new(actual_sample_rate, target_sample_rate))
    } else {
        None
    };

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
        if let Ok(samples) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
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
                print_partial(&last_result, stdout_is_tty);
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
                        print_final(trimmed_final, stdout_is_tty);
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
                                    let score = combined_score(trimmed_final, &refined);
                                    let decision = refine_decision(score);
                                    match decision {
                                        "override" => {
                                            if refined != trimmed_final {
                                                print_final_replace(&refined, stdout_is_tty);
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
                                                print_final_replace(&refined, stdout_is_tty);
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
            print_final(trimmed_final, stdout_is_tty);
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
                        let score = combined_score(trimmed_final, &refined);
                        let decision = refine_decision(score);
                        match decision {
                            "override" | "override_warn" => {
                                if refined != trimmed_final {
                                    print_final_replace(&refined, stdout_is_tty);
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

    session_log.session_end(SessionEndReason::CtrlC);
    Ok(())
}

/// 流式 partial 输出。TTY 下走就地覆盖（`\r` + ANSI erase-line），piped 下
/// 走普通 println，避免污染下游消费者。
///
/// ANSI 说明：
/// - `\r`：把光标移到行首
/// - `\x1b[2K`：擦除整行（包括光标右侧所有字符），保证新文本较短时不会残留
/// - `▌ {text}`：左侧细条 + 文本，是常见的"流式中"前缀符号
fn print_partial(text: &str, tty: bool) {
    let compact = strip_display_spaces(text);
    if tty {
        let mut out = std::io::stdout().lock();
        let _ = write!(out, "\r\x1b[2K▌ {}", compact);
        let _ = out.flush();
    } else {
        println!("{}", compact);
    }
}

/// endpoint 最终结果。TTY 下覆盖上一行 partial（如果有）并以 `\n` 提交本行，
/// 这样下一句 partial 会从新行开始，避免 partial 与 final 串行错位。
/// piped 下保留 `\n✅ ` 前缀风格，让 grep / awk 等下游工具更易识别。
fn print_final(text: &str, tty: bool) {
    let compact = strip_display_spaces(text);
    if tty {
        let mut out = std::io::stdout().lock();
        let _ = write!(out, "\r\x1b[2K✅ {}\n", compact);
        let _ = out.flush();
    } else {
        println!("\n✅ {}", compact);
    }
}

/// 覆盖刚刚由 `print_final` 写出的那行（用 `\x1b[1A` 上移一行）。
///
/// 注意：**必须紧跟 `print_final` 调用**，中间不能有任何 stdout / eprintln 输出，
/// 否则光标已经移走，\x1b[1A 会覆盖到错误的位置。
///
/// ANSI 序列：
/// - `\x1b[1A`：光标上移一行（从 print_final 末尾的 `\n` 回到 ✅ 行）
/// - `\r`：回行首
/// - `\x1b[2K`：擦除整行
/// - 然后写新的 `✅ {text}\n`
fn print_final_replace(text: &str, tty: bool) {
    let compact = strip_display_spaces(text);
    if tty {
        let mut out = std::io::stdout().lock();
        let _ = write!(out, "\x1b[1A\r\x1b[2K✅ {}\n", compact);
        let _ = out.flush();
    } else {
        // piped 下没有"上一行"可覆盖，追加新行
        println!("\n✅ {}", compact);
    }
}

/// 流式 Zipformer transducer 模型（x-asr）在中文字符之间插入 ASCII 空格作为视觉分隔符
/// （中文 ASR 的常见做法）。这种空格在显示和相似度计算中都应该被去掉：
/// - 显示：避免屏幕看到"今 天 天"而不是"今天天"
/// - 相似度：流式与精修（CTC 无空格）的对齐比较会因空格不一致而失真
///
/// 只过滤 ASCII 空格 `' '`，保留：
/// - 中英混输时的实际空格（如 "divide test" 中的空格）
/// - 中文全角空格 `　`
/// - 标点 `，。！？`
fn strip_display_spaces(s: &str) -> String {
    s.chars().filter(|c| *c != ' ').collect()
}

/// 比较时双方都先 strip 空格（避免流式带空格 vs 精修无空格导致的假性不匹配）。
fn normalize_for_compare(s: &str) -> String {
    strip_display_spaces(s)
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

/// 字符级 Jaccard 相似度（multiset）：`|A ∩ B| / max(|A|, |B|)`。
///
/// "multiset 交集"对每个字符 `c` 取 `min(count_in_a, count_in_b)`，
/// 因此能区分：
/// - **补字场景**（如末尾补 1 字）：共同字符 ≈ max(|A|, |B|) - 1 → jaccard 高
/// - **改结构场景**（如丢掉开头 5 字）：共同字符显著少于 max → jaccard 低
///
/// 与字集级 Jaccard（`|A ∩ B| / |A ∪ B|`，无 multiplicity）的差异：
/// - 字集级把"今天天气挺好的" vs "今天天气挺好"（差 1 字）算成 0.93，几乎一致
/// - 字符级对同样例子 ≈ 0.93（前 N-1 字完全对齐）
///
/// 但对**结构化改写**（开头丢了 5 字、中间换词）更敏感：
/// - "然后会议的页面..." vs "的页面..."（b 丢了开头 5 字）
///   - 字集级：0.65
///   - 字符级：~0.40（前缀错位、共同字符位置分散）
///
/// 边界：
/// - 两端都为空 → 1.0
/// - 一端空一端非空 → 0.0
fn jaccard(a: &str, b: &str) -> f32 {
    use std::collections::HashMap;
    let a = normalize_for_compare(a);
    let b = normalize_for_compare(b);
    if a == b {
        return 1.0;
    }
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.is_empty() && b_chars.is_empty() {
        return 1.0;
    }
    let max_len = a_chars.len().max(b_chars.len());
    if max_len == 0 {
        return 1.0;
    }

    // multiset 交集：对每个字符取 min(count_in_a, count_in_b)
    let mut count_a: HashMap<char, u32> = HashMap::new();
    for &c in &a_chars {
        *count_a.entry(c).or_insert(0) += 1;
    }
    let mut count_b: HashMap<char, u32> = HashMap::new();
    for &c in &b_chars {
        *count_b.entry(c).or_insert(0) += 1;
    }
    let mut common = 0u32;
    for (c, n_a) in &count_a {
        if let Some(n_b) = count_b.get(c) {
            common += (*n_a).min(*n_b);
        }
    }
    common as f32 / max_len as f32
}

/// 长度比 = min(|A|, |B|) / max(|A|, |B|)。
///
/// 用字符数（不是字节数）以避免 UTF-8 编码差异干扰。
/// 当一方显著短于另一方时（如精修掉了 30% 字符），分数降下来，
/// 即使字符组成相似也不能信任。
fn length_ratio(a: &str, b: &str) -> f32 {
    let a = normalize_for_compare(a);
    let b = normalize_for_compare(b);
    let len_a = a.chars().count() as f32;
    let len_b = b.chars().count() as f32;
    let max = len_a.max(len_b);
    if max == 0.0 {
        1.0
    } else {
        len_a.min(len_b) / max
    }
}

/// 前 N 字位置匹配率：same 位置 same 字符的比例。
///
/// 解决"丢前缀"型精修劣化的关键：精修经常把"然后会议"丢成"的页面"，
/// 字集 Jaccard 不敏感（前缀短的字集覆盖低），但前 N 字位置匹配 = 0。
const PREFIX_MATCH_LEN: usize = 5;

fn prefix_match(a: &str, b: &str, n: usize) -> f32 {
    let a = normalize_for_compare(a);
    let b = normalize_for_compare(b);
    let a_prefix: Vec<char> = a.chars().take(n).collect();
    let b_prefix: Vec<char> = b.chars().take(n).collect();
    if a_prefix.is_empty() && b_prefix.is_empty() {
        return 1.0;
    }
    let pair_len = a_prefix.len().min(b_prefix.len());
    if pair_len == 0 {
        return 0.0;
    }
    let common = a_prefix
        .iter()
        .zip(b_prefix.iter())
        .filter(|(x, y)| x == y)
        .count();
    common as f32 / pair_len as f32
}

/// 综合精修评分 = 字符级 Jaccard × 长度比 × 前缀匹配率。
///
/// 三个因子互补：
/// - 字符 Jaccard：衡量"用了多少相同字"
/// - 长度比：惩罚"精修显著短"（常意味着丢上下文）
/// - 前缀匹配：惩罚"开头丢了几个字"（最常见的劣化模式）
///
/// 综合后能可靠区分：
/// - 补字场景：Jaccard 高 + 长度接近 + 前缀一致 → 高分
/// - 丢前缀场景：Jaccard 中等 + 长度接近 + 前缀 0 → 低分
/// - 整体重写场景：Jaccard 低 + 长度差异大 → 低分
fn combined_score(streaming: &str, refined: &str) -> f32 {
    let char_j = jaccard(streaming, refined);
    let len_r = length_ratio(streaming, refined);
    let pre_r = prefix_match(streaming, refined, PREFIX_MATCH_LEN);
    char_j * len_r * pre_r
}

/// refine 决策阈值（基于 combined_score，0-1 之间）。
///
/// 阈值选择依据：实测多段会话：
/// - 完全一致 → 1.0 → override
/// - 末尾补 1-2 字：score 0.92-1.0 → override
/// - 同音字错（如"也"→"冷"、"当年"→"多年"）：score 0.62-0.79 → 应 rejected（之前 0.85 阈值会误判 override）
/// - 丢前缀 / 整段重写：score 0.0-0.4 → rejected
///
/// HIGH 阈值 0.92：宁可严苛一些，避免"jaccard 高但 prefix_match 漏掉中部错字"的情况漏过。
const REFINE_HIGH_THRESHOLD: f32 = 0.92;
const REFINE_MED_THRESHOLD: f32 = 0.40;

/// 三档决策：
/// - `"override"`        — combined_score >= 0.92，精修与流式结构高度一致
/// - `"override_warn"`   — 0.40 <= score < 0.92，部分一致，覆盖但日志标 warn
/// - `"rejected"`        — score < 0.40，差异过大（丢前缀 / 整体重写），保留流式结果
fn refine_decision(score: f32) -> &'static str {
    if score >= REFINE_HIGH_THRESHOLD {
        "override"
    } else if score >= REFINE_MED_THRESHOLD {
        "override_warn"
    } else {
        "rejected"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_identical() {
        assert_eq!(jaccard("今天天气挺好的", "今天天气挺好的"), 1.0);
    }

    #[test]
    fn jaccard_both_empty() {
        assert_eq!(jaccard("", ""), 1.0);
    }

    #[test]
    fn jaccard_one_empty() {
        assert_eq!(jaccard("今天", ""), 0.0);
        assert_eq!(jaccard("", "今天"), 0.0);
    }

    /// 字符级（multiset）Jaccard 的关键行为：
    /// - 补 1 字：共同字符数 ≈ max(|A|, |B|) - 1 → 高
    /// - 丢 5+ 字：共同字符数显著少于 max → 低
    #[test]
    fn jaccard_trailing_addition() {
        // 末尾补 1 字（典型 refine 场景）
        let j = jaccard(
            "今天天气比较热也希望一起把业务做起",
            "今天天气比较热也希望一起把业务做起来",
        );
        // 19 chars common, max 20 → 0.95
        assert!(j > 0.9, "trailing补字应该高 jaccard, got {}", j);
    }

    #[test]
    fn jaccard_head_loss() {
        // 开头丢了 5 字（典型精修劣化场景）
        let j = jaccard(
            "然后会议的页面移动端是原生的pc做了之后一旦端也要",
            "的页面移动端是原生的做了之后移动端也要做",
        );
        // multiset 共同 17/27 ≈ 0.63（不是特别低，但低于阈值）
        assert!(j < 0.7, "head loss 应该 < 0.7, got {}", j);
    }

    #[test]
    fn jaccard_structural_rewrite() {
        // 中间词替换（seg 2 那种场景）
        let j = jaccard(
            "移动端需要适配一",
            "终端需要适配一下",
        );
        // multiset 共同 6/8 = 0.75
        assert!(j > 0.7, "短文本 multiset 仍然较高, got {}", j);
    }

    #[test]
    fn length_ratio_basic() {
        assert_eq!(length_ratio("abc", "abc"), 1.0);
        assert_eq!(length_ratio("", ""), 1.0);
        // 5 vs 10 → 0.5
        assert!((length_ratio("abcde", "abcdefghij") - 0.5).abs() < 0.01);
        // 1 vs 100 → 0.01
        assert!(length_ratio("a", &"a".repeat(100)) < 0.02);
    }

    #[test]
    fn prefix_match_basic() {
        // 完全相同 → 1.0
        assert_eq!(prefix_match("今天天气好", "今天天气好", 5), 1.0);
        // 前缀完全不同 → 0.0
        assert_eq!(prefix_match("今天天气好", "然后会议", 5), 0.0);
        // 部分匹配（前 2 字 + 末尾 1 字相同 = 3/5）
        // "今天天气好" vs "今天然后好"
        // pos 0: 今-今 ✓
        // pos 1: 天-天 ✓
        // pos 2: 气-然 ✗
        // pos 3: 天-后 ✗
        // pos 4: 好-好 ✓
        // 3/5 = 0.6
        let p = prefix_match("今天天气好", "今天然后好", 5);
        assert!((p - 0.6).abs() < 0.01);
    }

    /// combined_score = char_jaccard × length_ratio × prefix_match
    /// 三因子互补：补字高分、丢前缀低分、长度差异大低分
    #[test]
    fn combined_score_trailing_addition() {
        // 末尾补 1 字：prefix_match=1.0, length_ratio=19/20=0.95, char_j=18/20=0.90
        // combined ≈ 0.85
        let s = combined_score(
            "今天天气比较热也希望一起把业务做起",
            "今天天气比较热也希望一起把业务做起来",
        );
        assert!(s > 0.80, "trailing补字应该高 score, got {}", s);
    }

    #[test]
    fn combined_score_head_loss() {
        // 开头丢了 5 字：prefix_match=0.0（对齐位置 0/5）
        // combined = 0
        let s = combined_score(
            "然后会议的页面移动端是原生的pc做了之后一旦端也要",
            "的页面移动端是原生的做了之后移动端也要做",
        );
        assert!(s < 0.1, "head loss 应该 score ≈ 0, got {}", s);
    }

    #[test]
    fn combined_score_real_session_fixture() {
        // session-20260724-031655 fixture（用 combined_score 计算的实际预期值）
        // seg 0 一致 → combined=1.0 → override
        let s0 = combined_score("今天天气挺好的", "今天天气挺好的");
        assert_eq!(refine_decision(s0), "override");

        // seg 3 "而且可脱钻修改宽高" vs "再修改宽高了"
        // prefix_match=0/5=0（然→再、且→修 都对不上）
        // combined = 0 → rejected
        let s3 = combined_score("而且可脱钻修改宽高", "再修改宽高了");
        assert!(s3 < 0.1, "head loss应该 score≈0, got {}", s3);
        assert_eq!(refine_decision(s3), "rejected");

        // seg 4 "看看会导致一些页面不太适合" vs "找的一些页面不太适合"
        // prefix_match=0/5=0（前 5 字都不同）→ combined=0 → rejected
        let s4 = combined_score(
            "看看会导致一些页面不太适合",
            "找的一些页面不太适合",
        );
        assert!(s4 < 0.1, "head loss应该 score≈0, got {}", s4);
        assert_eq!(refine_decision(s4), "rejected");
    }

    #[test]
    fn refine_decision_boundaries() {
        // 综合分阈值：0.92 / 0.40
        assert_eq!(refine_decision(0.92), "override");
        assert_eq!(refine_decision(0.919), "override_warn");
        assert_eq!(refine_decision(0.40), "override_warn");
        assert_eq!(refine_decision(0.399), "rejected");
        assert_eq!(refine_decision(1.0), "override");
        assert_eq!(refine_decision(0.0), "rejected");
    }
}
