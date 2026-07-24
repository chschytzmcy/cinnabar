//! 屏幕显示工具：partial/final 输出 + 文本归一化。
//!
//! 处理三件事：
//! 1. TTY vs piped 两种输出模式
//! 2. ITN（中文数字 → 阿拉伯数字）调用
//! 3. 流式 x-asr 字符间空格的 strip
//!
//! ## ANSI 序列说明
//! - `\r`：回行首
//! - `\x1b[2K`：擦除整行（光标右侧）
//! - `\x1b[1A`：光标上移一行
//!
//! 设计约束：print_final_replace 必须**紧跟** print_final 调用，
//! 中间不能有任何 stdout/eprintln 输出，否则 `\x1b[1A` 会覆盖错行。

use std::io::Write;

use crate::itn;

/// 流式 partial 输出。
///
/// TTY 下用 `\r\x1b[2K` 原地覆盖（不滚屏），piped 下用 println。
/// ITN 在 print 之前应用，JSONL 日志保留原始 ASR 输出。
pub fn print_partial(text: &str, tty: bool) {
    let compact = itn::itn(&strip_display_spaces(text));
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
pub fn print_final(text: &str, tty: bool) {
    let compact = itn::itn(&strip_display_spaces(text));
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
pub fn print_final_replace(text: &str, tty: bool) {
    let compact = itn::itn(&strip_display_spaces(text));
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
pub fn strip_display_spaces(s: &str) -> String {
    s.chars().filter(|c| *c != ' ').collect()
}

/// 比较时双方都先 strip 空格（避免流式带空格 vs 精修无空格导致的假性不匹配）。
pub fn normalize_for_compare(s: &str) -> String {
    strip_display_spaces(s)
}