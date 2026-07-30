//! wav_source: 把本地 WAV 文件送进与 cpal 同一的 crossbeam 通道。
//!
//! 严格格式：仅 PCM s16le / 16 kHz / 单声道。不支持的格式立即 anyhow 报错。
//! 异步：单独线程持续 try_send，主线程 join 后保证所有 chunk 都已入队。
//!
//! 调用方拿到 `JoinHandle` 后即可进入与麦克风路径完全相同的 `while running { rx.recv... }`
//! 主循环；读完一次后线程退出，`Sender` 被 drop，rx 端会开始收到 `Err(Disconnected)`，
//! 此时把 `running` 置 false 让主循环退出即可复用现有 flush/drain 块。

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Sender;
use hound::{SampleFormat, WavReader};
use std::path::Path;
use std::thread::{self, JoinHandle};

/// 默认 chunk 大小：30 ms @ 16 kHz = 480 samples。
/// 与 cpal 默认 buffer 量级一致，让 VAD/ASR 看到的节奏与真实麦克风相似。
pub const DEFAULT_CHUNK_SAMPLES: usize = 480;

/// 严格格式校验后读取 wav，按 `chunk_samples` 一批一批发到 `tx`。
///
/// 公开的纯同步函数，便于单元测试；异步入口 `spawn` 内部调它。
///
/// 返回读到的总样本数。
pub fn read_chunks(path: &Path, chunk_samples: usize, tx: &Sender<Vec<f32>>) -> Result<u64> {
    let mut reader = WavReader::open(path)
        .with_context(|| format!("打开 wav 文件失败: {}", path.display()))?;
    let spec = reader.spec();

    if spec.sample_rate != 16_000
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != SampleFormat::Int
    {
        return Err(anyhow!(
            "不支持的 wav 格式：要求 PCM s16le / 16 kHz / 单声道，实际 {} Hz / {} 通道 / {} bit {:?}",
            spec.sample_rate,
            spec.channels,
            spec.bits_per_sample,
            spec.sample_format
        ));
    }

    let mut total: u64 = 0;
    let mut buf: Vec<f32> = Vec::with_capacity(chunk_samples);

    for sample in reader.samples::<i16>() {
        let s = sample.with_context(|| format!("读取 wav 样本失败: {}", path.display()))?;
        // i16 / i16::MAX 归一化到 [-1.0, 1.0]，与 cpal f32 输入同量级。
        buf.push(s as f32 / i16::MAX as f32);
        if buf.len() >= chunk_samples {
            tx.send(std::mem::take(&mut buf))
                .map_err(|_| anyhow!("audio channel 已关闭（消费端先退出了？）"))?;
            buf.reserve(chunk_samples);
        }
        total = total.saturating_add(1);
    }

    if !buf.is_empty() {
        tx.send(buf)
            .map_err(|_| anyhow!("audio channel 已关闭（消费端先退出了？）"))?;
    }

    Ok(total)
}

/// 在后台线程里跑 `read_chunks`；返回 `JoinHandle<u64>`。
///
/// 调用方在主循环退出后 `join`，确保最后一批 chunk 已入队再走 flush 路径。
pub fn spawn<P: AsRef<Path>>(
    path: P,
    tx: Sender<Vec<f32>>,
    chunk_samples: usize,
) -> JoinHandle<Result<u64>> {
    let path = path.as_ref().to_path_buf();
    thread::spawn(move || read_chunks(&path, chunk_samples, &tx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{WavSpec, WavWriter};
    use tempfile::NamedTempFile;

    /// 把一段 mono 16-bit s16 写到 temp wav 文件，返回路径。
    /// `samples` 中每个 i16 都会被原样写入。
    fn write_temp_wav(samples: &[i16]) -> NamedTempFile {
        let tmp = NamedTempFile::new().expect("创建 tempfile");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(tmp.path(), spec).expect("创建 wav");
        for &s in samples {
            writer.write_sample(s).expect("写入样本");
        }
        writer.finalize().expect("finalize wav");
        tmp
    }

    /// 通过 unbounded channel 接住所有 chunk，方便断言。
    fn collect(samples: &[i16], chunk_samples: usize) -> (u64, Vec<Vec<f32>>) {
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let tmp = write_temp_wav(samples);
        let total =
            read_chunks(tmp.path(), chunk_samples, &tx).expect("read_chunks 应成功");
        // tx drop 后 rx 收到 Disconnected；用 try_iter 拿走所有剩余值。
        drop(tx);
        let chunks: Vec<Vec<f32>> = rx.try_iter().collect();
        (total, chunks)
    }

    #[test]
    fn total_sample_count_matches_input() {
        let samples: Vec<i16> = (0..8000).map(|i| (i % 1000) as i16).collect();
        let (total, _) = collect(&samples, 480);
        assert_eq!(total, 8000);
    }

    #[test]
    fn chunk_sizes_divisible_input() {
        let samples: Vec<i16> = vec![0; 960]; // 恰好 2 个完整 chunk
        let (total, chunks) = collect(&samples, 480);
        assert_eq!(total, 960);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 480);
        assert_eq!(chunks[1].len(), 480);
    }

    #[test]
    fn tail_chunk_carries_remainder() {
        let samples: Vec<i16> = vec![0; 1000]; // 480 + 480 + 40
        let (total, chunks) = collect(&samples, 480);
        assert_eq!(total, 1000);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 480);
        assert_eq!(chunks[1].len(), 480);
        assert_eq!(chunks[2].len(), 40);
    }

    #[test]
    fn normalization_max_sample_maps_to_one() {
        let samples = vec![i16::MAX, i16::MIN, 0];
        let (total, chunks) = collect(&samples, 480);
        assert_eq!(total, 3);
        let flat: Vec<f32> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
        // i16::MIN / i16::MAX ≈ -1.0000305（不对称的数学现象，因为 |MIN| > |MAX|），
        // 1e-3 的容差足够反映"近 ±1.0"的语义，又不会因为浮点细节放过真错误。
        assert!(
            (flat[0] - 1.0).abs() < 1e-3,
            "i16::MAX -> ~1.0, got {}",
            flat[0]
        );
        assert!(
            (flat[1] + 1.0).abs() < 1e-3,
            "i16::MIN -> ~-1.0, got {}",
            flat[1]
        );
        assert_eq!(flat[2], 0.0);
    }

    #[test]
    fn rejects_stereo_wav() {
        // 立体声是 interleaved：每帧必须写 左+右 两个样本，否则 hound 的 finalize 会
        // 返回 UnfinishedSample（数据样本数与声道数不匹配）。
        let tmp = NamedTempFile::new().expect("tempfile");
        let spec = WavSpec {
            channels: 2,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(tmp.path(), spec).expect("wav");
        writer.write_sample(0i16).unwrap(); // L
        writer.write_sample(0i16).unwrap(); // R
        writer.finalize().unwrap();

        let (tx, _rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let err = read_chunks(tmp.path(), 480, &tx).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("不支持的 wav 格式"),
            "应提示格式不匹配，实际: {}",
            msg
        );
    }

    #[test]
    fn rejects_44100_wav() {
        let tmp = NamedTempFile::new().expect("tempfile");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(tmp.path(), spec).expect("wav");
        writer.write_sample(0i16).unwrap();
        writer.finalize().unwrap();

        let (tx, _rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let err = read_chunks(tmp.path(), 480, &tx).unwrap_err();
        assert!(format!("{:#}", err).contains("不支持的 wav 格式"));
    }

    /// 测试 spawn 真的能 join 拿到总样本数。
    #[test]
    fn spawn_returns_total_on_join() {
        let samples: Vec<i16> = vec![123; 16000]; // 1 秒 @ 16 kHz
        let tmp = write_temp_wav(&samples);
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let handle = spawn(tmp.path(), tx, DEFAULT_CHUNK_SAMPLES);
        let total = handle.join().expect("join").expect("read_chunks ok");
        drop(rx);
        assert_eq!(total, 16000);
    }
}