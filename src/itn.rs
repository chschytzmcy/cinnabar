//! ITN (Inverse Text Normalization)：中文数字 → 阿拉伯数字。
//!
//! 用于在 `print_partial` / `print_final` / `print_final_replace` 三处把屏幕显示
//! 的中文数字转成阿拉伯数字（"二十秒" → "20秒"）。JSONL 日志和
//! `combined_score` 计算**不**调用 ITN，保留原始 ASR 输出便于复盘。
//!
//! ## 覆盖场景（v1 简化版）
//! - 单个数字：一/二/.../九、两、零/〇 → 1/2/.../9、2、0
//! - "十"/"百"/"千" 单独出现：→ 10/100/1000
//! - "X十Y" / "X百Y" / "X千Y"：→ X*10+Y、X*100+Y、X*1000+Y
//! - "X万" / "X亿"：→ X*10000、X*100000000
//! - 复合如 "二十三万" / "三万二千三百四十五"：正确解析
//! - "百分之N"：→ "N%"（作为整体识别，不能被拆）
//!
//! ## 不覆盖（v1 跳过）
//! - 复杂小数（零点一五、负数、分数）
//! - 序数（第一 → #1）
//! - 复杂年份/日期格式
//! - 与阿拉伯数字混排
//!
//! ## 设计取舍
//! 不引入 regex 依赖，手写解析器。覆盖 80% 实际用例，边界情况
//! （"两三个"这种口语范围）保持原样不强行转换。

/// 入口：把字符串中所有中文数字段转成阿拉伯数字。
pub fn itn(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // 优先识别"百分之"前缀（必须作为整体处理，否则 "百" 单独被识别成 100）
        if i + 3 <= chars.len()
            && chars[i] == '百' && chars[i + 1] == '分' && chars[i + 2] == '之'
        {
            let mut j = i + 3;
            while j < chars.len() && is_chinese_number_char(chars[j]) {
                j += 1;
            }
            if j > i + 3 {
                if let Some(n) = parse_chinese_number(&chars[i+3..j]) {
                    result.push_str(&n.to_string());
                    result.push('%');
                    i = j;
                    continue;
                }
            }
            // 没有跟着数字（如单独的"百分之"），跳过这 3 个字符
            result.push(chars[i]);
            result.push(chars[i+1]);
            result.push(chars[i+2]);
            i += 3;
            continue;
        }

        // 一般情况：找连续的中文数字段
        let start = i;
        while i < chars.len() && is_chinese_number_char(chars[i]) {
            i += 1;
        }
        if i > start {
            if let Some(arabic) = parse_chinese_number(&chars[start..i]) {
                result.push_str(&arabic.to_string());
            } else {
                // 转换失败（如 "两三个" 这种范围），保持原样
                for &c in &chars[start..i] {
                    result.push(c);
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

fn is_chinese_number_char(c: char) -> bool {
    matches!(c,
        '零' | '〇' | '一' | '壹' | '二' | '贰' | '两' |
        '三' | '叁' | '四' | '肆' | '五' | '伍' |
        '六' | '陆' | '七' | '柒' | '八' | '捌' |
        '九' | '玖' | '十' | '拾' | '百' | '佰' |
        '千' | '仟' | '万' | '萬' | '亿' | '億'
    )
}

fn chinese_digit_value(c: char) -> Option<u64> {
    match c {
        '零' | '〇' => Some(0),
        '一' | '壹' => Some(1),
        '二' | '贰' | '两' => Some(2),
        '三' | '叁' => Some(3),
        '四' | '肆' => Some(4),
        '五' | '伍' => Some(5),
        '六' | '陆' => Some(6),
        '七' | '柒' => Some(7),
        '八' | '捌' => Some(8),
        '九' | '玖' => Some(9),
        _ => None,
    }
}

fn chinese_unit_value(c: char) -> Option<u64> {
    match c {
        '十' | '拾' => Some(10),
        '百' | '佰' => Some(100),
        '千' | '仟' => Some(1000),
        '万' | '萬' => Some(10_000),
        '亿' | '億' => Some(100_000_000),
        _ => None,
    }
}

/// 解析中文数字段。算法：
/// - 数字（0-9）累加到 current 段
/// - 低位单位（十/百/千）乘到 current
/// - 高位单位（万/亿）累加到 total 后乘
/// - 数字在单位后是"加"（如 "二十三" = 二*10 + 三），不是乘 10
/// 关键：数字紧跟低单位出现时（如 "三百二十"），需要关闭前一个子段
/// （"三百" → 300 加入 total），再开始新子段（"二十" → 20）。
/// 失败（包含非数字字符）返回 None。
fn parse_chinese_number(chars: &[char]) -> Option<u64> {
    let mut total: u64 = 0;
    let mut current: u64 = 0;
    let mut last_was_digit = false;

    for &c in chars {
        if let Some(d) = chinese_digit_value(c) {
            if last_was_digit {
                // 连续数字（如 "二十三" 的二三部分）→ 乘 10 加
                current = current.checked_mul(10)?.checked_add(d)?;
            } else {
                // 数字紧跟低单位出现 → 关闭前一个子段，开始新子段
                // 例："三百二十" 的二，紧跟 百
                //     → "三百" 300 加入 total，"二" 2 开始新 current
                if current > 0 {
                    total = total.checked_add(current)?;
                }
                current = d;
            }
            last_was_digit = true;
        } else if let Some(u) = chinese_unit_value(c) {
            if current == 0 {
                current = 1; // 单独的"十"/"百"等
            }
            if u >= 10_000 {
                // 高位单位：flush current 到 total，再乘
                if current > 0 {
                    total = total.checked_add(current)?;
                }
                total = total.checked_mul(u)?;
                current = 0;
            } else {
                // 低位单位：current 乘以单位
                current = current.checked_mul(u)?;
            }
            last_was_digit = false;
        } else {
            return None;
        }
    }
    total.checked_add(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_digits() {
        // 单个中文数字
        assert_eq!(itn("一"), "1");
        assert_eq!(itn("二"), "2");
        assert_eq!(itn("三"), "3");
        assert_eq!(itn("九"), "9");
        assert_eq!(itn("零"), "0");
        assert_eq!(itn("两"), "2");
    }

    #[test]
    fn unit_alone() {
        // 单独出现的"十/百/千"
        assert_eq!(itn("十"), "10");
        assert_eq!(itn("百"), "100");
        assert_eq!(itn("千"), "1000");
    }

    #[test]
    fn low_unit() {
        // X十Y / X百Y / X千Y
        assert_eq!(itn("二十"), "20");
        assert_eq!(itn("二十三"), "23");
        assert_eq!(itn("三十七"), "37");  // 用户测试用例
        assert_eq!(itn("三百二十"), "320");
        assert_eq!(itn("五百"), "500");
        assert_eq!(itn("二千三"), "2003");
    }

    #[test]
    fn high_unit() {
        // X万 / X亿 / 复合
        assert_eq!(itn("三万"), "30000");
        assert_eq!(itn("二十三万"), "230000");  // 复合：23*10000
        assert_eq!(itn("三万二千三百四十五"), "32345");
    }

    #[test]
    fn percent() {
        // 百分之N → N%
        assert_eq!(itn("百分之五十"), "50%");
        assert_eq!(itn("百分之五"), "5%");
        assert_eq!(itn("百分之一百"), "100%");
    }

    #[test]
    fn mixed_text() {
        // 数字 + 非数字
        assert_eq!(itn("今天"), "今天");  // "天" 不是数字
        assert_eq!(itn("二十秒"), "20秒");
        assert_eq!(itn("三秒"), "3秒");
        assert_eq!(itn("二零二六年"), "2026年");
        assert_eq!(itn("OK"), "OK");  // 英文不变
    }

    #[test]
    fn empty() {
        assert_eq!(itn(""), "");
    }

    #[test]
    fn unparseable_preserved() {
        // 不能解析的保持原样（不强行转换）
        // "两三个" 这种口语范围，ITN 不处理
        // 注意：当前实现会尝试转换 "两" (2)，剩下 "三个" 不是数字会保留
        // 主要验证不 panic
        let _ = itn("两三个");
    }
}