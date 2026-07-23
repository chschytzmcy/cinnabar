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
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::bounded;
use ffi::OnlineRecognizer;
use logger::{SessionEndReason, SessionLog, SessionStartInfo};
use resampler::LinearResampler;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vad::EndpointDetector;

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
}

fn main() -> Result<()> {
    let args = Args::parse();

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

    let recognizer = OnlineRecognizer::new(
        &args.model_dir.join("encoder.int8.onnx").to_string_lossy(),
        &args.model_dir.join("decoder.int8.onnx").to_string_lossy(),
        &args.model_dir.join("tokens.txt").to_string_lossy(),
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
    let verbose = args.verbose;

    let audio_stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            if verbose {
                eprintln!("[DEBUG] 音频回调: 接收到 {} 个样本", data.len());
            }
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
            if verbose {
                eprintln!("[DEBUG] 音频回调: 混音后 {} 个样本", mono_data.len());
            }
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
        vad_threshold: 0.01,
        min_silence_ms: 1200,
        min_speech_ms: 500,
    });

    let mut resampler = if use_resampler {
        Some(LinearResampler::new(actual_sample_rate, target_sample_rate))
    } else {
        None
    };

    let mut endpoint_detector = EndpointDetector::new(0.01, target_sample_rate, 1.2, 0.5);
    let mut last_result = String::new();
    let mut last_printed = String::new();
    let mut last_update_time = std::time::Instant::now();
    let mut partial_seq: u64 = 0;
    let mut samples_in_session: u64 = 0;

    while running.load(Ordering::Relaxed) {
        if let Ok(samples) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
            if args.verbose {
                eprintln!("[DEBUG] 主循环: 接收到 {} 个样本", samples.len());
            }
            if samples.is_empty() {
                continue;
            }

            if args.verbose {
                eprintln!("[DEBUG] 主循环: 开始重采样");
            }
            let samples_16k = if let Some(ref mut r) = resampler {
                r.resample(&samples)
            } else {
                samples
            };
            if args.verbose {
                eprintln!("[DEBUG] 主循环: 重采样后 {} 个样本", samples_16k.len());
            }

            // 检查重采样后的数据是否为空
            if samples_16k.is_empty() {
                continue;
            }

            if args.verbose {
                eprintln!("[DEBUG] 主循环: 调用 accept_waveform");
            }
            stream.accept_waveform(target_sample_rate as i32, &samples_16k);
            samples_in_session = samples_in_session.saturating_add(samples_16k.len() as u64);

            if args.verbose {
                eprintln!("[DEBUG] 主循环: 检查 is_ready");
            }
            while recognizer.is_ready(&stream) {
                if args.verbose {
                    eprintln!("[DEBUG] 主循环: 调用 decode");
                }
                recognizer.decode(&mut stream);
            }

            if args.verbose {
                eprintln!("[DEBUG] 主循环: 获取结果");
            }
            let result = recognizer.get_result(&stream);
            let trimmed = result.trim();

            let mut printed_just_now = false;
            if !trimmed.is_empty() && trimmed != last_result {
                last_result = trimmed.to_string();
                last_update_time = std::time::Instant::now();
                partial_seq = partial_seq.saturating_add(1);
                // 文本变更即落日志（printed=false），便于事后回放 ASR 模型的
                // 贪心解码轨迹；真正打印到终端时再补一条 printed=true。
                session_log.partial(partial_seq, trimmed, samples_in_session);
            }

            // 如果超过 500ms 没有新内容，输出当前结果。
            // 用 last_printed 记录上次实际打印过的内容，避免 partial 结果稳定
            // 在多个 500ms 窗口里被反复打印（例如 "今" 持续不变时）。
            if !last_result.is_empty()
                && last_result != last_printed
                && last_update_time.elapsed().as_millis() > 500
            {
                println!("{}", last_result);
                last_printed = last_result.clone();
                last_result.clear();
                printed_just_now = true;
            }
            if printed_just_now {
                // 已打印的 partial 也补一条日志，便于事后区分"模型已识别"和"用户可见"。
                session_log.partial(partial_seq, &last_printed, samples_in_session);
            }

            if args.verbose {
                eprintln!("[DEBUG] 主循环: 检查 endpoint");
            }
            let is_endpoint = endpoint_detector.accept_waveform(&samples_16k);
            if args.verbose {
                eprintln!("[DEBUG] 主循环: endpoint = {}", is_endpoint);
            }
            if is_endpoint {
                // 记录 endpoint 事件本身，包含语音/静音时长，便于复盘静音阈值。
                let (speech_ms, silence_ms) = endpoint_snapshot(&endpoint_detector);
                session_log.endpoint_detected(speech_ms, silence_ms);

                if args.verbose {
                    eprintln!("[DEBUG] 主循环: endpoint 为 true，获取最终结果");
                }
                let final_result = recognizer.get_result(&stream);
                if args.verbose {
                    eprintln!(
                        "[DEBUG] 主循环: 获取到最终结果，长度 = {}",
                        final_result.len()
                    );
                }
                if !final_result.trim().is_empty() {
                    let trimmed_final = final_result.trim();
                    // endpoint 触发的最终结果：仅在和已打印内容不同时输出，
                    // 避免和上面 500ms debounce 窗口重复。
                    if trimmed_final != last_printed {
                        println!("\n✅ {}", trimmed_final);
                        last_printed = trimmed_final.to_string();
                    }
                    session_log.final_result(trimmed_final);
                }
                if args.verbose {
                    eprintln!("[DEBUG] 主循环: 准备销毁并重建流");
                }
                // 绕过 sherpa-onnx 1.12.9 OnlineStreamReset 路径下的状态损坏：
                // 直接销毁当前 OnlineStream，再用 recognizer 重新创建一个。
                // 这样下一次 accept_waveform 走的是全新实例的干净状态。
                stream = recognizer.create_stream();
                endpoint_detector.reset();
                if args.verbose {
                    eprintln!("[DEBUG] 主循环: 流已重建，检测器已重置");
                }
            }
            if args.verbose {
                eprintln!("[DEBUG] 主循环: 本次循环结束");
            }
        }
    }

    session_log.session_end(SessionEndReason::CtrlC);
    Ok(())
}

/// 读出当前 endpoint_detector 的语音/静音累计时长（毫秒）。
/// EndpointDetector 把这两个值封装在私有字段里，所以这里只能走 accept_waveform
/// 之后的内部状态：speech_samples 表示累计语音样本数，silence_samples 表示
/// 累计静音样本数（reset 后从 0 开始累加）。
fn endpoint_snapshot(d: &EndpointDetector) -> (u64, u64) {
    let sr = d.sample_rate() as u64;
    let speech_samples = d.speech_samples() as u64;
    let silence_samples = d.silence_samples() as u64;
    let speech_ms = if sr == 0 { 0 } else { speech_samples * 1000 / sr };
    let silence_ms = if sr == 0 { 0 } else { silence_samples * 1000 / sr };
    (speech_ms, silence_ms)
}
