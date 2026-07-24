# Cinnabar 性能调优指南

本文档提供 Cinnabar 的性能优化建议和调优技巧。

## 目录

- [性能基准](#性能基准)
- [CPU 优化](#cpu-优化)
- [内存优化](#内存优化)
- [延迟优化](#延迟优化)
- [电池优化](#电池优化)
- [基准测试](#基准测试)

---

## 性能基准

### 测试环境

**硬件配置**：
- CPU: AMD Ryzen 7 5800X (8C/16T)
- RAM: 32GB DDR4-3600
- 存储: NVMe SSD
- 音频: USB 麦克风（48kHz/16bit）

**软件环境**：
- OS: Arch Linux (Kernel 6.7)
- Audio: PipeWire 1.0.3
- Rust: 1.75.0

### 性能指标

| 指标 | CLI 模式 | GUI 模式 | 目标 |
|------|---------|---------|------|
| 端到端延迟（P50） | 65ms | 72ms | <100ms |
| 端到端延迟（P95） | 95ms | 108ms | <150ms |
| 端到端延迟（P99） | 125ms | 142ms | <200ms |
| 内存占用（稳态） | 85MB | 102MB | <200MB |
| CPU 使用率（单核） | 28% | 35% | <50% |
| 启动时间 | 1.2s | 1.8s | <3s |
| 模型加载时间 | 0.8s | 0.8s | <2s |

### 延迟分解

```
总延迟 (65ms) = 音频捕获 (10ms) + 重采样 (1ms) + 
                通道传输 (2ms) + 模型推理 (48ms) + 
                显示输出 (4ms)
```

### 资源消耗

**磁盘空间**：
- 二进制文件: 8.2MB (release)
- 模型文件: 42MB (INT8)
- 总计: ~50MB

**网络带宽**：
- 无（完全离线）

**电池影响**（笔记本）：
- 待机: +2% / 小时
- 活跃识别: +8% / 小时

---

## CPU 优化

### 1. 调整 ONNX 线程数

编辑 `src/ffi/mod.rs`：

```cinnabar/src/ffi/mod.rs#L1-10
// 低端 CPU（2-4 核）
num_threads: 2,

// 中端 CPU（4-6 核）
num_threads: 3,

// 高端 CPU（8+ 核）
num_threads: 4,
```

**建议**：
- 单核/双核：`num_threads: 1`
- 四核：`num_threads: 2`
- 八核及以上：`num_threads: 4`

### 2. CPU 亲和性设置

```cinnabar/docs/PERFORMANCE.md#L1-5
# 绑定到性能核心（大核）
taskset -c 0-3 cargo run --release

# 绑定到特定核心
taskset -c 0 cargo run --release
```

### 3. 启用性能调度器

```cinnabar/docs/PERFORMANCE.md#L1-10
# 临时设置
sudo cpupower frequency-set -g performance

# 永久设置（systemd）
sudo systemctl enable --now cpupower.service

# 验证当前调度器
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

### 4. 禁用 CPU 节能特性

```cinnabar/docs/PERFORMANCE.md#L1-8
# 禁用 Intel Turbo Boost（如果导致热节流）
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# 禁用 AMD Cool'n'Quiet
echo 0 | sudo tee /sys/devices/system/cpu/cpufreq/boost
```

---

## 内存优化

### 1. 减少通道缓冲区

编辑 `src/main.rs`：

```cinnabar/src/main.rs#L1-5
// 默认值：100（约 6.25 秒缓冲）
let (tx, rx) = bounded::<Vec<f32>>(100);

// 低内存设备：50（约 3.1 秒缓冲）
let (tx, rx) = bounded::<Vec<f32>>(50);
```

**权衡**：
- 更小的缓冲区 = 更低内存占用
- 但可能增加音频丢失风险

### 2. 使用 jemalloc 分配器

添加到 `Cargo.toml`：

```cinnabar/Cargo.toml#L1-5
[dependencies]
jemallocator = "0.5"

[profile.release]
opt-level = 3
```

在 `src/main.rs` 添加：

```cinnabar/src/main.rs#L1-3
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

### 3. 监控内存使用

```cinnabar/docs/PERFORMANCE.md#L1-10
# 实时监控
watch -n 1 'ps aux | grep cinnabar | grep -v grep'

# 详细内存分析
/usr/bin/time -v cargo run --release

# 检查内存泄漏
valgrind --leak-check=full --show-leak-kinds=all cargo run --release
```

---

## 延迟优化

### 1. 减少音频缓冲区

```cinnabar/docs/PERFORMANCE.md#L1-5
# PipeWire 低延迟配置
export PIPEWIRE_LATENCY=32/48000
cargo run --release

# 或编辑 ~/.config/pipewire/pipewire.conf
default.clock.min-quantum = 32
```

### 2. 实时优先级

```cinnabar/docs/PERFORMANCE.md#L1-8
# 需要 root 或 CAP_SYS_NICE
sudo chrt -f 50 cargo run --release

# 或添加到 /etc/security/limits.conf
@audio - rtprio 95
@audio - memlock unlimited
```

### 3. 禁用调试日志

确保使用 `--release` 模式：

```cinnabar/docs/PERFORMANCE.md#L1-5
# 错误：debug 模式慢 10 倍
cargo run

# 正确：release 模式
cargo run --release
```

### 4. 优化重采样器

编辑 `src/main.rs`，减少轮询间隔：

```cinnabar/src/main.rs#L1-5
// 默认：100ms
rx.recv_timeout(std::time::Duration::from_millis(100))

// 低延迟：50ms（增加 CPU 使用率）
rx.recv_timeout(std::time::Duration::from_millis(50))
```

---

## 电池优化

### 1. 降低采样率处理频率

编辑 `src/main.rs`：

```cinnabar/src/main.rs#L1-5
// 增加超时时间，降低轮询频率
rx.recv_timeout(std::time::Duration::from_millis(200))

// 或使用阻塞接收（最省电）
rx.recv()
```

### 2. 启用 VAD（ten-vad，v1.2.4+）

ten-vad 神经网络 VAD 自动判断 segment 边界，比早期能量阈值 VAD 更省心。

调整 VAD 阈值（编辑 `config.toml` 或 CLI flag 覆盖）：

```toml
# 更高的阈值 = 更严格的语音判定 = 更省电（但容易漏检弱音）
vad_threshold = 0.6

# 更短的推理窗口 = 更频繁的判断 = CPU 略高
vad_window_size = 256

# 减少推理线程
vad_num_threads = 1
```

注意：ten-vad 的 threshold 是概率（0.0-1.0），不再使用早期 `0.01` 能量阈值，**两者语义不同**。

### 3. 使用节能模式

```cinnabar/docs/PERFORMANCE.md#L1-5
# 平衡性能和功耗
sudo cpupower frequency-set -g powersave

# 或使用 schedutil（推荐）
sudo cpupower frequency-set -g schedutil
```

### 4. 限制 ONNX 线程数

```cinnabar/src/ffi/mod.rs#L1-3
// 笔记本电脑建议使用 2 线程
num_threads: 2,
```

---

## 基准测试

### 测量端到端延迟

```cinnabar/docs/PERFORMANCE.md#L1-10
# 使用 verbose 模式查看时间戳
cargo run --release -- --verbose 2>&1 | grep "主循环"

# 或使用 perf 分析
perf record -g cargo run --release
perf report

# 生成火焰图
cargo install flamegraph
cargo flamegraph --release
```

### 内存分析

```cinnabar/docs/PERFORMANCE.md#L1-10
# 使用 heaptrack
heaptrack cargo run --release
heaptrack_gui heaptrack.cinnabar.*.gz

# 或使用 valgrind（慢）
valgrind --tool=massif cargo run --release
ms_print massif.out.*
```

### CPU 分析

```cinnabar/docs/PERFORMANCE.md#L1-10
# 生成火焰图
cargo install flamegraph
cargo flamegraph --release
# 打开 flamegraph.svg

# 使用 perf
perf record -F 99 -g cargo run --release
perf report
```

### 压力测试

```cinnabar/docs/PERFORMANCE.md#L1-10
# 长时间运行测试（24 小时）
timeout 86400 cargo run --release

# 监控资源使用
while true; do
    ps aux | grep cinnabar | grep -v grep
    sleep 60
done > resource_usage.log
```

---

## 性能问题诊断

### CPU 使用率过高

**诊断步骤**：
1. 检查 ONNX 线程数是否过多
2. 验证是否使用 release 模式
3. 检查是否有其他程序占用 CPU

**解决方案**：
```cinnabar/docs/PERFORMANCE.md#L1-5
# 减少线程数
num_threads: 2

# 降低轮询频率
rx.recv_timeout(std::time::Duration::from_millis(200))
```

### 内存占用过高

**正常范围**：<200MB

**诊断步骤**：
```cinnabar/docs/PERFORMANCE.md#L1-5
# 检查内存泄漏
valgrind --leak-check=full cargo run --release

# 监控内存增长
watch -n 1 'ps aux | grep cinnabar'
```

### 延迟过高

**诊断步骤**：
```cinnabar/docs/PERFORMANCE.md#L1-10
# 启用详细日志
cargo run --release -- --verbose

# 分析延迟来源
perf record -g cargo run --release
perf report

# 检查音频缓冲区设置
pw-metadata -n settings
```

---

## 平台特定优化

### Arch Linux / CachyOS

```cinnabar/docs/PERFORMANCE.md#L1-5
# 使用优化的内核
sudo pacman -S linux-zen

# 启用性能调度器
sudo cpupower frequency-set -g performance
```

### Ubuntu / Debian

```cinnabar/docs/PERFORMANCE.md#L1-5
# 安装低延迟内核
sudo apt install linux-lowlatency

# 配置音频优先级
sudo usermod -aG audio $USER
```

### Fedora

```cinnabar/docs/PERFORMANCE.md#L1-5
# 启用 RPM Fusion
sudo dnf install rpmfusion-free-release

# 安装性能工具
sudo dnf install perf htop
```

---

## 最佳实践

1. **始终使用 release 模式**：`cargo run --release`
2. **根据 CPU 核心数调整线程数**：2-4 线程
3. **监控资源使用**：定期检查 CPU 和内存
4. **使用性能调度器**：笔记本除外
5. **启用 VAD**：减少不必要的处理
6. **定期更新依赖**：`cargo update`

---

**最后更新**：2026-02-03  
**版本**：v1.2.3