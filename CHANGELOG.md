# Changelog

所有对用户行为可见的改动都记录在这里。版本遵循 [Semantic Versioning](https://semver.org/)。

## [2.0.0] - 2026-07-24

### Breaking changes

- **非流式 ASR 模型更换**：`sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03` → `sherpa-onnx-paraformer-zh-2023-09-14`。
  Paraformer attention decoder 不再"过度摘要"（之前 Zipformer CTC 经常丢"应该/也/就是"等连接词）。
  CER 从 1.74% (CTC) 升至 ~6.5% (Paraformer zh-2023-09-14)，但段落完整性大幅改善。
- **流式 ASR 模型更换**：`sherpa-onnx-streaming-paraformer-bilingual-zh-en` → `sherpa-onnx-x-asr-960ms-streaming-zipformer-transducer-zh-en-punct-int8-2026-06-05`。
  体积 227MB → 127MB（int8），内置标点恢复，960ms chunk。
- **流式输出字符间有空格**：x-asr Zipformer transducer 在中文字符之间插入 ASCII 空格（如"今 天 天"）。
  显示层已 strip（`src/display.rs::strip_display_spaces`），JSONL 日志保留原始输出便于复盘差异。
- **refine 评分算法重写**：从单一字符 jaccard 改为**三因子综合评分**
  `combined_score = char_jaccard × length_ratio × prefix_match`（见 `src/refine_score.rs`）。
  阈值从 0.85/0.40 改为 **0.92/0.40**，减少"同音字错"被误判 override（如"也"→"冷"）。
- **热词功能默认禁用**：`docs/hotword.md`（33 条人名/地名/品牌）作为资源保留，
  但 v1.12.9 vendored lib 的 Paraformer 实现只支持 `greedy_search`，
  与 `hotwords_file` 要求的 `modified_beam_search` 冲突。升级到 v1.13+ 可启用。
- **模块拆分**（内部重构，无接口变化）：`src/main.rs` 拆出独立模块
  - `src/display.rs`（3 个 print_* + strip_display_spaces）
  - `src/refine_score.rs`（3 因子评分 + 13 个测试）
  - `src/itn.rs`（中文数字归一化）

### Added

- **VAD flush 修复长停顿不切段**：静音帧调 `vad.flush()` 强制 ten-vad 吐出 pending segment。
  修复前：用户停顿 > 1.2s 期间不切段；修复后：1.2s 静音自动切段。
- **ITN 中文数字归一化**（仅在 print 阶段调用，JSONL 不变）：
  "二十"→"20"，"三秒"→"3秒"，"百分之五十"→"50%"，"二零二六年"→"2026年"。
  复杂小数/负数/序数不覆盖（v1 简化）。
- **JSONL 事件 schema 扩展**：
  - `endpoint` 事件改用 `SegmentCommitInfo` 结构（`segment_id`/`start_sample`/`samples`/`duration_ms`）
  - `refine` 事件加 `score` 字段（0-1，combined_score 值）
  - `session_start` info 加 `streaming_model` 字段（如"Paraformer" / "SenseVoice"）
- **CLI flag 新增**：
  - `--debounce-ms` 流式 partial 输出节流间隔（默认 150ms）
  - `--vad-threshold` / `--vad-min-silence-ms` / `--vad-min-speech-ms` 覆盖 VAD 参数
  - `--offline-decoding` 改 decode 方法（实际 v1.12.9 限制仅 greedy_search）
  - `--no-itn` 关闭 ITN

### Changed

- 流式 partial 渲染：节流改为 150ms（之前 500ms），跟手感显著提升
- 默认 TTY 刷新策略：流式 partial 用 `\r\x1b[2K▌` 原地刷新，不再每行追加
- 段错误修复：流式 recognizer 流销毁从 `reset()` 改为 `create_stream()`（绕开 1.12.9 reset 崩溃）
- seg 0 之后流式 partial 的 printed/printed 拆分为两个独立事件（decoded/printed）

### Removed

- 旧的 Zipformer CTC 非流式模型目录 `models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/`
- 根目录的旧 streaming-paraformer 文件（`encoder.int8.onnx` / `decoder.int8.onnx` / `encoder.onnx` / `decoder.onnx` / `tokens.txt`）
- 旧 `vad::VadDetector`（替换为 ten-vad 直接调用）
- 旧 `combined_score` 的旧算法（单字符 jaccard）

## [1.2.3] - 2026-07-11

- 修复 endpoint 后段错误（destroy+recreate 流替代 reset）

## [1.2.0] - 2026-05-15

- 接入 sherpa-onnx 流式 + 非流式 ASR 双轨
- 集成 ten-vad
- JSONL 会话日志
- 配置文件支持（TOML）

## [1.0.0] - 2024-03-10

- 初始 release：流式 Paraformer + 简单能量 VAD
