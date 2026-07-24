//! 流式 ASR 与非流式 ASR 精修结果之间的相似度评分。
//!
//! 三因子综合评分（combined_score）：
//! - 字符 Jaccard（multiset）：衡量"用了多少相同字"
//! - 长度比：惩罚"精修显著短"（常意味着丢上下文）
//! - 前缀匹配：惩罚"开头丢了几个字"（最常见的劣化模式）
//!
//! 三档决策（基于 combined_score）：
//! - `>= 0.92` → override（信任精修，覆盖屏幕）
//! - `0.40 <= score < 0.92` → override_warn（覆盖但日志标 warn）
//! - `< 0.40` → rejected（不覆盖，保留流式结果）

/// 前缀匹配长度（5 字 = 中文约 0.3 秒音频）。
const PREFIX_MATCH_LEN: usize = 5;

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
pub fn jaccard(a: &str, b: &str) -> f32 {
    use std::collections::HashMap;
    let a = crate::display::normalize_for_compare(a);
    let b = crate::display::normalize_for_compare(b);
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
pub fn length_ratio(a: &str, b: &str) -> f32 {
    let a = crate::display::normalize_for_compare(a);
    let b = crate::display::normalize_for_compare(b);
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
pub fn prefix_match(a: &str, b: &str, n: usize) -> f32 {
    let a = crate::display::normalize_for_compare(a);
    let b = crate::display::normalize_for_compare(b);
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
pub fn combined_score(streaming: &str, refined: &str) -> f32 {
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
pub const REFINE_HIGH_THRESHOLD: f32 = 0.92;
pub const REFINE_MED_THRESHOLD: f32 = 0.40;

/// 三档决策：
/// - `"override"`        — combined_score >= 0.92，精修与流式结构高度一致
/// - `"override_warn"`   — 0.40 <= score < 0.92，部分一致，覆盖但日志标 warn
/// - `"rejected"`        — score < 0.40，差异过大（丢前缀 / 整体重写），保留流式结果
pub fn refine_decision(score: f32) -> &'static str {
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