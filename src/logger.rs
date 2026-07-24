use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 一行一条 JSON 对象的会话日志。
///
/// 落盘路径优先级：
/// 1. CLI 传入的 `--log-file`
/// 2. `$XDG_DATA_HOME/cinnabar/sessions/session-<UTC>.jsonl`
/// 3. `~/.local/share/cinnabar/sessions/session-<UTC>.jsonl`
///
/// 设计取舍：JSONL 比 CSV/TSV 更适合嵌套结构（partial 文本、配置字典等），
/// 同时保留人眼可读性，便于事后 `jq` / `grep` 直接捞数据。
pub struct SessionLog {
    writer: BufWriter<std::fs::File>,
    path: PathBuf,
}

impl SessionLog {
    pub fn open(override_path: Option<&Path>) -> Result<Self> {
        let path = match override_path {
            Some(p) => p.to_path_buf(),
            None => default_session_path()?,
        };

        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .with_context(|| format!("创建日志目录失败: {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("打开日志文件失败: {}", path.display()))?;

        Ok(Self {
            writer: BufWriter::new(file),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn emit(&mut self, ev: &LogEvent) {
        // 日志写入失败不应阻塞识别主流程：吞掉错误，仅 eprintln 提示。
        match serde_json::to_string(ev) {
            Ok(line) => {
                if let Err(e) = writeln!(self.writer, "{}", line) {
                    eprintln!("[LOGGER] 写入日志失败: {}", e);
                    return;
                }
                let _ = self.writer.flush();
            }
            Err(e) => eprintln!("[LOGGER] 序列化日志失败: {}", e),
        }
    }

    pub fn session_start(&mut self, info: &SessionStartInfo) {
        self.emit(&LogEvent::SessionStart {
            ts_ms: now_ms(),
            info: info.clone(),
        });
    }

    /// 流式 partial 结果。`seq` 自增，便于乱序/丢失分析。
    ///
    /// `printed` 字段区分两种事件：
    /// - `false`：ASR 解码出文本那一刻落盘（即便后续节流期内不再变化）
    /// - `true`：节流窗口到点、文本真的打印到屏幕那一刻落盘
    ///
    /// 这条区分让事后能看出"模型识别的轨迹"和"用户看到的轨迹"差异。
    pub fn partial(&mut self, seq: u64, text: &str, samples_so_far: u64, printed: bool) {
        self.emit(&LogEvent::Partial {
            ts_ms: now_ms(),
            seq,
            text: text.to_string(),
            samples_so_far,
            printed,
        });
    }

    /// ten-vad 切出的 segment 被 commit 时记录。
    /// `segment_id` 是本次会话内 segment 的自增编号；
    /// `start_sample` 是 ten-vad 给出的相对 16 kHz buffer 起点；
    /// `duration_ms` = `seg.samples.len() * 1000 / sample_rate`。
    pub fn endpoint_detected(&mut self, info: &SegmentCommitInfo) {
        self.emit(&LogEvent::Endpoint {
            ts_ms: now_ms(),
            info: info.clone(),
        });
    }

    /// endpoint 触发后输出的最终结果（带 ✅ 前缀的那条）。
    pub fn final_result(&mut self, text: &str) {
        self.emit(&LogEvent::Final {
            ts_ms: now_ms(),
            text: text.to_string(),
        });
    }

    /// 离线 ASR refine 覆盖事件 —— 流式 final 出炉后再被非流式精修时记录。
    /// `streaming_text` 是屏幕上先打印的版本，`refined_text` 是非流式覆盖后的版本。
    /// 两者一致时也记（用于复盘"精修没改字"的比例）。
    pub fn refine(&mut self, streaming_text: &str, refined_text: &str) {
        self.emit(&LogEvent::Refine {
            ts_ms: now_ms(),
            streaming_text: streaming_text.to_string(),
            refined_text: refined_text.to_string(),
        });
    }

    /// 任意错误事件，不致命但值得记录。
    #[allow(dead_code)]
    pub fn warn(&mut self, message: &str) {
        self.emit(&LogEvent::Warn {
            ts_ms: now_ms(),
            message: message.to_string(),
        });
    }

    pub fn session_end(&mut self, reason: SessionEndReason) {
        self.emit(&LogEvent::SessionEnd {
            ts_ms: now_ms(),
            reason,
        });
        let _ = self.writer.flush();
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionStartInfo {
    pub mode: String,
    pub model_dir: String,
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub resampled: bool,
    pub vad_model_path: String,
    pub vad_threshold: f32,
    pub min_silence_ms: u32,
    pub min_speech_ms: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SegmentCommitInfo {
    pub segment_id: u64,
    pub start_sample: i32,
    pub samples: u32,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    CtrlC,
    #[allow(dead_code)]
    CleanExit,
    #[allow(dead_code)]
    Error,
}

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum LogEvent {
    SessionStart {
        ts_ms: u64,
        info: SessionStartInfo,
    },
    Partial {
        ts_ms: u64,
        seq: u64,
        text: String,
        samples_so_far: u64,
        printed: bool,
    },
    Endpoint {
        ts_ms: u64,
        info: SegmentCommitInfo,
    },
    Final {
        ts_ms: u64,
        text: String,
    },
    Refine {
        ts_ms: u64,
        streaming_text: String,
        refined_text: String,
    },
    #[allow(dead_code)]
    Warn {
        ts_ms: u64,
        message: String,
    },
    SessionEnd {
        ts_ms: u64,
        reason: SessionEndReason,
    },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn default_session_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| dirs_home().map(|h| h.join(".local").join("share")))
        .context("无法解析用户主目录")?;

    let stamp = chrono_like_utc_stamp();
    Ok(base
        .join("cinnabar")
        .join("sessions")
        .join(format!("session-{}.jsonl", stamp)))
}

/// 不引入 chrono 依赖，手写一个 UTC 时间戳：YYYYMMDD-HHMMSS。
fn chrono_like_utc_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    utc_stamp_from_epoch_secs(secs)
}

fn utc_stamp_from_epoch_secs(mut secs: u64) -> String {
    // 简化版 Gregorian 转换，足够生成文件名。
    let s = (secs % 60) as u8;
    secs /= 60;
    let mi = (secs % 60) as u8;
    secs /= 60;
    let h = (secs % 24) as u8;
    secs /= 24;
    let mut days = secs as i64; // 自 1970-01-01 起的天数

    let year = {
        let mut y = 1970i64;
        loop {
            let dy = if is_leap(y) { 366 } else { 365 };
            if days < dy {
                break y;
            }
            days -= dy;
            y += 1;
        }
    };
    let mdays = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u8;
    for &dm in &mdays {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    let day = (days as u8) + 1;
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, h, mi, s
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_format() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        let s = utc_stamp_from_epoch_secs(1704067200);
        assert_eq!(s, "20240101-000000");
    }

    #[test]
    fn stamp_after_leap_day() {
        // 2024-03-01 00:00:00 UTC = 1709251200
        let s = utc_stamp_from_epoch_secs(1709251200);
        assert_eq!(s, "20240301-000000");
    }
}