# Cinnabar 故障排除指南

本文档提供 Cinnabar 常见问题的诊断和解决方案。

## 目录

- [编译问题](#编译问题)
- [运行时问题](#运行时问题)
- [音频问题](#音频问题)
- [识别问题](#识别问题)
- [性能问题](#性能问题)
- [GUI 模式问题](#gui-模式问题)

---

## 编译问题

### 错误：sherpa-onnx-c-api 库未找到

**症状**：
```
error: linking with `cc` failed
= note: /usr/bin/ld: cannot find -lsherpa-onnx-c-api
```

**原因**：模型文件未下载或库路径配置错误

**解决方案**：
```bash
# 1. 下载模型和库
./setup_models.sh

# 2. 验证库文件存在
ls -la models/lib/libsherpa-onnx-c-api.so

# 3. 清理并重新构建
cargo clean
cargo build --release
```

### 错误：CMake 版本过低

**症状**：
```
CMake 3.20 or higher is required. You are running version 3.16
```

**解决方案**：
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install cmake

# Arch Linux
sudo pacman -S cmake

# 验证版本
cmake --version
```

### 错误：Rust 版本不兼容

**症状**：
```
error: package requires rustc 1.70 or newer
```

**解决方案**：
```bash
# 更新 Rust
rustup update stable

# 验证版本
rustc --version
```

---

## 运行时问题

### 错误：未找到默认音频设备

**症状**：
```
Error: 未找到默认输入设备
```

**诊断步骤**：
```bash
# 1. 检查麦克风硬件
arecord -l

# 2. 测试录音
arecord -d 5 test.wav && aplay test.wav

# 3. 检查 PipeWire 状态
systemctl --user status pipewire pipewire-pulse
```

**解决方案**：
```bash
# 列出可用设备
cargo run --release -- --list-devices

# 指定设备
cargo run --release -- --device 0
cargo run --release -- --device-name "麦克风名称"
```

### 错误：权限被拒绝（GUI 模式）

**症状**：
```
Error: Failed to build virtual device
Permission denied: /dev/uinput
```

**原因**：用户不在 input 组

**解决方案**：
```bash
# 添加用户到 input 组
sudo usermod -aG input $USER

# 注销并重新登录，或临时生效
newgrp input

# 验证权限
ls -l /dev/uinput
```

### 错误：模型加载失败

**症状**：
```
Error: 创建识别器失败
```

**诊断步骤**：
```bash
# 1. 验证模型文件完整性
sha256sum models/*.onnx

# 2. 检查文件权限
ls -la models/

# 3. 检查磁盘空间
df -h
```

**解决方案**：
```bash
# 重新下载模型
rm -rf models
./setup_models.sh
```

### 错误：共享库未找到

**症状**：
```
error while loading shared libraries: libsherpa-onnx-c-api.so
```

**解决方案**：
```bash
# 方案 1：使用 cargo run（自动设置 rpath）
cargo run --release

# 方案 2：手动设置 LD_LIBRARY_PATH
export LD_LIBRARY_PATH=$PWD/models/lib:$LD_LIBRARY_PATH
./target/release/cinnabar

# 方案 3：安装到系统路径
sudo cp models/lib/libsherpa-onnx-c-api.so /usr/local/lib/
sudo ldconfig
```

---

## 音频问题

### 问题：无声音输入

**诊断步骤**：
```bash
# 1. 检查麦克风硬件
arecord -l

# 2. 测试录音
arecord -d 5 -f cd test.wav && aplay test.wav

# 3. 检查 PipeWire 状态
systemctl --user status pipewire pipewire-pulse

# 4. 查看音频设备
pactl list sources short
```

**解决方案**：
```bash
# 重启 PipeWire
systemctl --user restart pipewire pipewire-pulse

# 调整麦克风增益
alsamixer
# 按 F4 选择捕获设备，调整音量
```

### 问题：音频断断续续

**可能原因**：
1. CPU 负载过高
2. 音频缓冲区太小
3. USB 麦克风供电不足

**解决方案**：
```bash
# 1. 关闭其他占用 CPU 的程序
htop

# 2. 增加 PipeWire 缓冲区
pw-metadata -n settings 0 clock.force-quantum 1024

# 3. 使用有源 USB Hub（如果是 USB 麦克风）
```

### 问题：回声或噪音

**解决方案**：
```bash
# 1. 启用噪音抑制（PipeWire）
pactl load-module module-echo-cancel

# 2. 调整麦克风增益
alsamixer

# 3. 环境优化
# - 使用指向性麦克风
# - 远离音箱和风扇
# - 距离麦克风 15-30cm
```

### 问题：采样率不匹配

**症状**：
```
⚠️  16kHz 不支持，使用默认配置: 48000 Hz, 2 声道（将启用重采样）
```

**说明**：这是正常行为，程序会自动启用软件重采样

**优化建议**：
```bash
# 如果设备支持 16kHz，可以在 PipeWire 配置中设置
# 编辑 ~/.config/pipewire/pipewire.conf.d/99-custom.conf
default.clock.rate = 16000
```

---

## 识别问题

### 问题：识别率低

**改进建议**：

**1. 环境优化**：
- 安静环境（<40dB 背景噪音）
- 距离麦克风 15-30cm
- 避免回声和混响

**2. 说话技巧**：
- 语速适中（不要太快）
- 发音清晰（避免含糊）
- 标准普通话（减少方言）

**3. 硬件升级**：
- 使用专业麦克风（非笔记本内置）
- USB 麦克风优于 3.5mm 接口
- 考虑麦克风阵列

### 问题：不识别英文

**说明**：Paraformer 是中英双语模型，但：
- 中文识别效果更好
- 纯英文句子可能识别率较低
- 中英混合效果最佳

**建议**：等待 Phase 3 的 Whisper 模型支持

### 问题：延迟过高（>200ms）

**诊断**：
```bash
# 启用详细日志
cargo run --release -- --verbose
```

**优化步骤**：
```bash
# 1. 检查 CPU 使用率
htop

# 2. 关闭其他占用 CPU 的程序

# 3. 使用性能模式
sudo cpupower frequency-set -g performance

# 4. 减少 ONNX 线程数（编辑 src/ffi/mod.rs）
num_threads: 2,  // 从 4 改为 2
```

### 问题：句子分割不准确

**说明**：自 v1.2.4 起，VAD 已从能量阈值升级到 sherpa-onnx 的 **ten-vad**（神经网络 VAD，由声网开源），不再使用 `EndpointDetector`。

**调整参数**（编辑 `config.toml` 或用 CLI flag 覆盖）：
```toml
# 提高灵敏度（更激进地判定为语音）：
vad_threshold = 0.3

# 缩短切 segment 所需的最长静音（更早断句）：
vad_min_silence_ms = 700

# 拉长最短语音长度（过滤掉更短的爆破音）：
vad_min_speech_ms = 700
```

CLI 覆盖示例：
```bash
cinnabar --vad-threshold 0.3 --vad-min-silence-ms 700
```

调整后查看 JSONL 日志里 `endpoint` 事件的 `duration_ms` 字段，确认切段时机是否符合预期。

### 问题：ten-vad 模型缺失

`VoiceActivityDetector` 创建失败，报错 `创建 ten-vad VoiceActivityDetector 失败（检查 --vad-model ...）`。

**解决**：
```bash
./setup_models.sh    # 会下载 ./models/ten-vad.onnx（约 324 KB）
ls -lh ./models/     # 确认 ten-vad.onnx 存在
```

或自定义路径：
```bash
cinnabar --vad-model /path/to/your/ten-vad.onnx
```

### 问题：ten-vad 阈值过高/过低

- 阈值太高（>0.7）：很多真实语音被判为静音，整句丢失。**降低** `vad_threshold` 到 0.3-0.5。
- 阈值太低（<0.2）：风扇、咳嗽声被误判为语音，segment 切得太碎。**提高** `vad_threshold` 到 0.5-0.7。

注意：ten-vad 的 threshold 是**概率**（0.0-1.0），不是早期能量 VAD 时代的能量值 0.01，**两者不可类比**。

### 问题：非流式 ASR 加载失败

启动时打印 `⚠️  非流式 ASR 加载失败，退回纯流式`，识别仍能跑但屏幕上的 final 不会被精修覆盖。

**检查**：
1. `models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/model.int8.onnx` 是否存在
2. `models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/tokens.txt` 是否存在
3. `config.toml` 的 `offline_model_path` / `offline_tokens_path` 是否与实际路径匹配
4. `provider`（默认 `cpu`）是否与硬件兼容

**手动拉取**：
```bash
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03.tar.bz2 \
     -O /tmp/offline.tar.bz2
mkdir -p models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03
tar -xjf /tmp/offline.tar.bz2 -C /tmp/
cp /tmp/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/*.onnx \
   /tmp/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/tokens.txt \
   models/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/
```

### 问题：refine 之后屏幕上的 final 没被覆盖

非流式返回的文本与流式**完全一致**时，代码不会触发 `print_final_replace`（避免无意义光标跳动）。日志里能看到 `refine` 事件的 `streaming_text == refined_text`，证明非流式确实跑了，只是没改字。

如果希望**每次都强制覆盖**（比如测试阶段），可以临时把 `print_final_replace` 的 `if refined != trimmed_final` 条件改成总是触发。

### 问题：refine 太慢（CPU 老 / 笔记本）

非流式 RTF 在官方文档里是 0.062（具体硬件未标注，按官方基准配置），老 CPU 实测可能 500ms–1s。

**缓解**：
- `config.toml` 里把 `offline_num_threads` 减到 1（单线程对小 segment 够用）
- 改用更轻量的模型：`offline_model_path = "./models/sherpa-onnx-paraformer-zh-small-2024-03-09/model.int8.onnx"`（78MB，RTF 0.076，但官方没有 WER 数据）
- 直接关掉：`enable_offline_refine = false` 或 `--no-offline-refine`，退回纯流式

### 问题：流式 vs 精修 文本不一致

正常情况下这正是 refine 的意义 —— 流式贪婪解码先出"差不多"的版本，离线重算修正几个字。

JSONL 里看具体差异：
```bash
jq -c 'select(.event=="refine")' ~/.local/share/cinnabar/sessions/*.jsonl
```

常见原因：
- 中英混输：流式把 `divide` 拼成 `d i v i d`，离线拼成 `divide` 或 `divide`
- 数字识别：流式把"二零二三"按字拆，离线可能合并成"2023"
- 同音字：流式选了"在/再"错的版本

---

## 性能问题

### 问题：CPU 使用率 100%

**可能原因**：
1. 音频采样率过高
2. ONNX 线程数过多
3. 系统资源不足

**解决方案**：
```bash
# 1. 强制使用 16kHz（编辑 src/main.rs）
# 2. 减少 ONNX 线程数（编辑 src/ffi/mod.rs）
num_threads: 2,

# 3. 检查系统资源
htop
free -h
```

### 问题：内存占用过高

**正常范围**：<200MB

**诊断**：
```bash
# 监控内存使用
watch -n 1 'ps aux | grep cinnabar'

# 检查内存泄漏
valgrind --leak-check=full cargo run --release
```

**解决方案**：
- 减少通道缓冲区大小（编辑 `src/main.rs`）
- 使用更小的模型（如果有）

### 问题：电池消耗快（笔记本）

**优化建议**：
```bash
# 1. 使用节能模式
sudo cpupower frequency-set -g powersave

# 2. 增加轮询间隔（编辑 src/main.rs）
rx.recv_timeout(std::time::Duration::from_millis(200))

# 3. 仅在需要时运行
```

---

## GUI 模式问题

### 问题：热键不响应

**诊断步骤**：
```bash
# 1. 检查热键是否被其他程序占用
# 2. 尝试不同的热键（编辑 config.toml）
hotkey = "F4"

# 3. 查看日志
cargo run --release -- --mode gui --verbose
```

### 问题：文本注入失败

**可能原因**：
1. 权限不足（/dev/uinput）
2. 目标应用不接受输入
3. Wayland 安全限制

**解决方案**：
```bash
# 1. 验证权限
ls -l /dev/uinput

# 2. 测试虚拟键盘
evtest /dev/uinput

# 3. 检查目标应用是否支持剪贴板粘贴
```

### 问题：悬浮窗不显示

**诊断步骤**：
```bash
# 1. 检查 Wayland 支持
echo $XDG_SESSION_TYPE

# 2. 查看错误日志
cargo run --release -- --mode gui 2>&1 | grep -i error

# 3. 验证 egui 依赖
cargo tree | grep egui
```

---

## 调试技巧

### 启用详细日志

```bash
# CLI 模式
cargo run --release -- --verbose

# GUI 模式
cargo run --release -- --mode gui --verbose
```

### 性能分析

```bash
# CPU 分析
cargo install flamegraph
cargo flamegraph --release

# 内存分析
cargo install heaptrack
heaptrack cargo run --release

# 延迟分析
cargo run --release -- --verbose 2>&1 | grep "主循环"
```

### 查看系统日志

```bash
# 查看段错误信息
sudo dmesg | tail -20 | grep segfault

# 查看音频相关日志
journalctl --user -u pipewire -f
```

---

## 获取帮助

如果以上方法无法解决问题，请：

1. **查看文档**：
   - [ROADMAP.md](ROADMAP.md) - 开发路线图和 FAQ
   - [AGENTS.md](../AGENTS.md) - 架构设计文档

2. **提交 Issue**：
   - 包含完整的错误信息
   - 提供系统信息（OS、Rust 版本、音频栈）
   - 附上 `--verbose` 模式的日志

3. **社区支持**：
   - GitHub Discussions
   - 相关技术论坛

---

**最后更新**：2026-02-03  
**版本**：v1.2.3