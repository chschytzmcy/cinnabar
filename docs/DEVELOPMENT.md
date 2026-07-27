# Development Guide

`cinnabar` 项目的开发指南。涵盖模块结构、构建/测试/检查命令、commit 规范、调试技巧。

## 项目概览

`cinnabar` 是一个流式中文语音转文字工具。架构由三个常驻/按需组件组成：
- **流式 ASR**（x-asr Zipformer transducer）— 实时跟手显示
- **VAD**（ten-vad）— 句子边界检测
- **非流式 ASR**（Paraformer）— 切段后精修覆盖

完整架构见 [ARCHITECTURE.md](./ARCHITECTURE.md)。

## 模块结构

```
src/
├── main.rs              主循环 + main() 入口                  (~800 行)
├── itn.rs               中文数字归一化（仅 print 阶段调用）   (~240 行)
├── display.rs           TTY/pipe 输出 + 文本归一化           (~75 行)
├── refine_score.rs      三因子相似度评分 + 三档决策         (~300 行)
├── config.rs            TOML 配置 + 默认值                 (~253 行)
├── logger.rs            JSONL 会话日志                    (~316 行)
├── vad.rs               ten-vad 包装（v1.12.9）            (~108 行)
├── ffi.rs               sherpa-onnx-c-api Rust 绑定        (~900 行)
├── recognizer.rs        GUI 模式识别引擎                  (~220 行)
├── resampler.rs         简单线性重采样                    (~60 行)
├── injector.rs          剪贴板 + uinput 文本注入           (~170 行)
├── wayland.rs           Wayland 客户端（活动窗口定位）     (~25 行)
└── gui/                 egui 桌面 UI                       (~200 行)
```

### 模块依赖图

```
                    ┌────────┐
                    │ main   │
                    └───┬────┘
          ┌───────────┬──┴──┬────────────┐
          ▼           ▼     ▼            ▼
     ┌────────┐  ┌──────┐  ┌─────┐  ┌────────┐
     │ display│  │logger│  │ vad │  │  ffi   │
     └────┬───┘  └──────┘  └──┬──┘  └──┬─────┘
          │                  │        │
          ▼                  │        ▼
     ┌─────────┐              │  ┌──────────┐
     │  itn   │              │  │recognizer│
     └─────────┘              │  └──────────┘
                              │
                              ▼
                        ┌────────────┐
                        │  resampler │
                        └────────────┘
                         
                  ┌─────────────────┐
                  │ refine_score    │（main 和 logger 都可能调用）
                  └─────────────────┘
```

## 常用命令

### 构建

```bash
# 标准 release 构建
cargo build --release

# 调试构建
cargo build

# 启用所有 warnings（CI 推荐）
RUSTFLAGS="-D warnings" cargo build --release
```

### 测试

```bash
# 运行所有测试
cargo test --release

# 只跑某个模块
cargo test --release --bin cinnabar itn
cargo test --release --bin cinnabar refine_score
cargo test --release --bin cinnabar config

# 跑单个测试
cargo test --release --bin cinnabar test_config_vad_fields_default

# 显示输出（用于调试失败的测试）
cargo test --release --bin cinnabar -- --nocapture
```

### 检查

```bash
# 格式化
cargo fmt
cargo fmt --check         # CI 用

# Clippy（lint）
cargo clippy --release
cargo clippy --release -- -D warnings  # CI 用，把 warning 当 error

# 文档
cargo doc --no-deps --open
```

### 运行

```bash
# 标准运行
cargo run --release

# 详细日志
cargo run --release -- --verbose

# 关闭非流式精修
cargo run --release -- --no-offline-refine

# 指定模型目录
cargo run --release -- --model-dir /path/to/models
```

## 调试技巧

### 查看 JSONL 会话日志

```bash
# 最新会话
ls -t ~/.local/share/cinnabar/sessions/*.jsonl | head -1 | xargs less

# 只看 refine 事件
jq -c 'select(.event=="refine")' \
   ~/.local/share/cinnabar/sessions/session-*.jsonl | tail

# 按决策过滤
jq -c 'select(.event=="refine" and .decision=="rejected")' \
   ~/.local/share/cinnabar/sessions/session-*.jsonl
```

### 常见调试场景

| 现象 | 查什么 |
|---|---|
| VAD 不切段 | 找 `partial` 事件，看 partials 之间间隔；找 `endpoint` 事件之间时长 |
| refine 总被拒绝 | `jq 'select(.event=="refine")'` 看 `score` 字段，应该 0.0~0.4 |
| 流式 partial 漏字 | 看 `samples_so_far` 是否递增（确认音频在流） |
| ITN 不工作 | 看 `partial_decoded` 文本是不是中文数字（ITN 只在 print 阶段，JSONL 不变） |

## Commit 规范

格式：
```
<type>(<scope>): <subject>

<body>
```

`<type>` 取值：
- `feat` 新功能
- `fix` 修 bug
- `refactor` 重构（无功能变化）
- `chore` 杂项（版本、依赖、构建）
- `docs` 文档
- `test` 测试
- `style` 格式（无逻辑变化）

`<scope>` 取值（可省略）：
- `itn` / `display` / `refine_score` / `vad` / `config` / `logger` / `ffi`
- `offline-asr` / `streaming-asr` / `refine-gating`
- `main`（主循环）

`<subject>` 中文一句话，≤ 50 字，不带句号。

示例：
```
feat(itn): 中文数字归一化
fix(vad): 静音帧调 flush 修复长停顿不切段
refactor: 提取 display + refine_score 到独立模块
chore(release): 1.2.3 → 2.0.0
```

## 添加新功能的流程

1. **设计**：考虑放哪个模块
   - 显示 → `display.rs`
   - 文本处理 → `itn.rs`（已规范化）
   - 相似度/决策 → `refine_score.rs`
   - 配置 → `config.rs`
   - 底层 ASR 绑定 → `ffi.rs`
   - 主循环逻辑 → `main.rs`
2. **实现**：在合适模块添加函数 + 单元测试
3. **测试**：`cargo test --release` + 跑实际场景验证
4. **commit + push**
5. **更新 CHANGELOG.md**（如果是用户可见变化）

## 模块边界

**Do:**
- 新功能在对应模块下加
- 单元测试放在模块内 `#[cfg(test)] mod tests`
- 跨模块调用用模块名 `display::print_partial` 显式限定
- 公共 API 写在 `pub fn`/`pub struct`

**Don't:**
- 跨模块访问私有字段（应该用 `pub(crate)`）
- 在 `main.rs` 写业务逻辑（应该放专门模块）
- 引入新依赖不加理由（每次依赖变化都在 commit 里说明）

## 性能

- **冷启动**：~5s（两个 ASR 模型 + VAD 加载）
- **热运行**：每音频帧 ~50ms CPU
- **内存**：~470MB（流式 162M + 非流式 233M + VAD 328K + 运行时 ~75MB）
- **延迟**：流式 50ms / 段（partial 节流后），非流式 200-500ms / 段

## 测试策略

### 单元测试
每个模块有 `#[cfg(test)] mod tests` 块。
- 数值函数：边界值 + 中间值
- 字符串处理：典型输入 + 边界（空、单字符、特殊字符）
- 决策函数：刚好在阈值上下的值

### 集成测试
当前没有自动化集成测试。手动测试：
- 跑一个完整 session
- 检查 JSONL 日志
- 用 `jq` 提取特定事件类型
- 确认决策覆盖正确率

### 添加新测试场景
- 写新功能时同时写测试
- 跑 `cargo test --release` 确认 100% 通过
- 修复 bug 时加回归测试

## 故障排查

| 错误 | 检查 |
|---|---|
| `error creating offline recognizer` | `model.int8.onnx` 路径，文件存在性，文件大小（应 200MB+） |
| `error creating online recognizer` | encoder/decoder/joiner 三个文件，文件名 `joiner.int8.onnx` 注意带 `.int8` |
| VAD 不切段 | 看 JSONL 里的 partial 时间戳间隔；如 > 5s 都没 endpoint，**VAD 卡了** |
| refine 总是 rejected | 看 `score` 字段，如 0.0 则 prefix_match=0（流式与精修开头不同） |
| 模型路径错 | 看启动日志里的 `🔧 非流式 ASR 已加载（Paraformer...）` 或 `⚠️ 加载失败` |

## 升级计划

未来可能做的：
- 升级到 sherpa-onnx 1.13+ 解锁 hotwords（需重新生成 c-api 绑定）
- 替换为 SenseVoice 非流式（更现代，但需更多绑定工作）
- 升级流式到 Whisper streaming（多语言，但体积大）
- 性能优化（CPU 端 ONNX Runtime 调优）

## 关键文件位置

| 文件 | 说明 |
|---|---|
| `Cargo.toml` | 项目元信息 + 依赖 |
| `setup_models.sh` | 模型下载脚本 |
| `src/main.rs` | 主循环 + main() |
| `src/ffi.rs` | sherpa-onnx-c-api Rust 绑定（最大文件） |
| `src/config.rs` | TOML 配置 + 默认值 |
| `docs/ARCHITECTURE.md` | 架构图 + 时序 |
| `docs/CHANGELOG.md` | 版本变更记录 |
| `docs/INSTALL.md` | 安装 |
| `docs/PERFORMANCE.md` | 性能基线 |
| `docs/TROUBLESHOOTING.md` | 故障排查 |