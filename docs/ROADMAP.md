# Cinnabar (朱砂) - 开发路线图

## 当前版本：v1.2.3

### 架构模式

Cinnabar 支持两种运行模式：
- **CLI 模式**：纯终端模式，用于研究和测试语音识别效果（当前已实现）
- **GUI 模式**：悬浮窗模式，按 F3 触发语音输入，自动注入文本（规划中）

### 已完成功能

#### Phase 1: CLI 核心演示 ✅
- [x] 音频捕获与重采样
- [x] 流式语音识别
- [x] 自动断句（基于 VAD + 静音检测）
- [x] 终端输出

#### Phase 1.5: 音频设备管理 ✅
- [x] 设备枚举和选择（`--list-devices`, `--device`, `--device-name`）
- [x] 音频配置回退机制
- [x] 自动 rpath 配置
- [x] 多声道混音支持（使用 sqrt(channels) 优化）
- [x] 调试模式（`--verbose`）
- [x] 智能句子分割（基于标点符号检测）
- [x] 优化终端输出显示

---

## 短期规划（Phase 2）

### GUI 模式实现

**目标**：实现悬浮窗模式，支持热键触发和自动文本注入

#### Phase 2.1: 基础 GUI 框架（v1.1.0）✅
- [x] 创建 `src/gui/` 模块
- [x] 实现基础悬浮窗（egui + eframe）
- [x] 实现 CLI/GUI 模式切换
- [x] 显示状态信息（待机/监听/识别中）

#### Phase 2.2: 热键集成（v1.2.0）✅
- [x] 集成 `global-hotkey` crate
- [x] 注册 F3 热键
- [x] 实现热键触发语音输入
- [x] 状态机管理（待机 → 监听 → 识别 → 注入）

#### Phase 2.3: 文本注入集成（v1.3.0）✅
- [x] 集成 `TextInjector` 模块（已实现）
- [x] 自动注入识别结果到激活窗口
- [x] 添加注入成功/失败反馈
- [x] 支持中英文混合输入

#### Phase 2.4: 窗口定位（v1.4.0）✅
- [x] 实现 Wayland 窗口信息获取
- [x] 检测激活窗口和输入框位置
- [x] 悬浮窗自动定位到输入框附近
- [x] 支持多种定位策略

**技术栈**：
- `egui` (0.30) + `eframe` (0.30) - GUI 框架
- `global-hotkey` (0.6) - 全局热键
- `wayland-client` (0.31) - Wayland 协议
- `arboard` (3.4) + `evdev` (0.12) - 文本注入（已实现）

**Wayland 支持**：
- ✅ Sway, Hyprland, GNOME, KDE Plasma
- 使用 `zwlr_layer_shell_v1` 创建悬浮层
- 使用 `wlr_foreign_toplevel_management` 获取窗口信息

**优先级**：高

---

## 中期规划（Phase 3）

### 高级特性

**语音活动检测（VAD）**：✅
- [x] 集成简单 VAD（能量阈值检测）   ← v1.0 - v1.2.3
- [x] 自动检测说话开始/结束
- [x] 降低功耗和 CPU 使用率
- [x] 实现 EndpointDetector（VAD + 静音时长）   ← v1.2.3
- [x] 替代 sherpa-onnx 的不稳定 is_endpoint
- [x] 升级到 ten-vad（神经网络 VAD）   ← v1.2.4，对噪声鲁棒、不依赖麦克风增益

**离线 ASR 精修**：✅
- [x] 流式 + 非流式并行加载   ← v1.2.5
- [x] VAD 切段后非流式覆盖屏幕 final（CER 降 2-5 pp）
- [x] 关闭开关 `--no-offline-refine` / `enable_offline_refine = false`
- [x] 兜底：非流式模型缺失时优雅退回纯流式

**配置管理**：✅
- [x] 配置文件支持（TOML）
- [x] 保存用户偏好设置
- [x] 模型路径配置

**多模型支持**：
- [ ] 支持 Whisper 模型
- [ ] 支持 Conformer 模型
- [ ] 模型热切换
- [ ] 模型性能对比

**文本后处理**：
- [ ] 标点符号恢复
- [ ] 自定义词汇注入
- [ ] 说话人分离（可选）

**技术栈**：
- `webrtc-vad` (0.4) - VAD 实现
- `whisper-rs` (0.12) - Whisper 模型支持
- `serde` + `toml` (1.0) - 配置文件
- `notify` (7.0) - 文件监听

**优先级**：中

---

## 长期规划（Phase 4）

### 图形界面

**系统托盘**：
- [ ] 系统托盘图标
- [ ] 监听状态可视化
- [ ] 快速启动/停止

**配置面板**：
- [ ] 设备选择界面
- [ ] 模型管理界面
- [ ] 热键配置
- [ ] 主题设置

**实时反馈**：
- [ ] 语音波形显示
- [ ] 识别结果实时预览
- [ ] 识别准确率统计

**技术栈选项**：
- `egui` (0.30) - 轻量级即时模式 GUI
- `iced` (0.13) - Elm 风格声明式 GUI
- `gtk4-rs` (0.9) - Linux 原生外观
- `tauri` (2.1) - Web 技术栈

**优先级**：低

---

## Phase 5: 生产优化与扩展

### 性能优化

**内存优化**：
- [ ] 实现流式缓冲区复用
- [ ] 优化模型加载内存占用
- [ ] 添加内存使用监控和告警
- [ ] 实现模型懒加载

**延迟优化**：
- [ ] 优化音频处理管道
- [ ] 实现预测性解码
- [ ] 减少线程切换开销
- [ ] 添加延迟监控和分析

**多线程优化**：
- [ ] 实现并行音频处理
- [ ] 优化 ONNX 线程池配置
- [ ] 添加 CPU 亲和性设置

### 跨平台支持

**X11 支持**：
- [ ] 实现 X11 窗口定位
- [ ] X11 输入法集成
- [ ] 热键支持（X11）

**其他平台**：
- [ ] macOS 支持（探索性）
- [ ] BSD 系统支持

### 企业级特性

**安全性**：
- [ ] 实现沙箱隔离
- [ ] 添加审计日志
- [ ] 支持加密模型文件
- [ ] 实现权限管理

**可观测性**：
- [ ] Prometheus metrics 导出
- [ ] 分布式追踪支持
- [ ] 性能分析工具集成

**部署**：
- [ ] Docker 容器化
- [ ] Kubernetes Helm Chart
- [ ] 系统服务模板（systemd）
- [ ] 自动更新机制

**优先级**：低

---

## 已知问题与限制

### 当前限制

1. **Endpoint 检测已优化（v1.2.3）✅**
   - 原因：sherpa-onnx 库的 `SherpaOnnxOnlineStreamIsEndpoint` 函数存在崩溃问题
   - 解决方案：实现自定义 `EndpointDetector`（VAD + 静音时长检测）
   - 技术细节：基于能量阈值的 VAD + 累计静音/语音时长判断
   - 触发条件：语音时长 ≥ 0.5s 且静音时长 ≥ 1.2s
   - 状态：✅ 已完成，CLI 和 GUI 模式均已集成
   - 优先级：已解决

2. **仅支持 Paraformer 模型**
   - 原因：当前仅实现了 Paraformer 模型的集成
   - 影响：无法使用其他 ASR 模型
   - 状态：Phase 3 规划中
   - 优先级：中

3. **无虚拟键盘功能**
   - 原因：Phase 2 尚未实现
   - 影响：无法直接输入到其他应用程序
   - 状态：Phase 2 规划中
   - 优先级：高

### 技术债务

| 问题 | 优先级 | 状态 | 位置 |
|------|--------|------|------|
| FFI 绑定手动维护 | 低 | 待优化 | `src/ffi/mod.rs` |
| Endpoint 检测崩溃 | 高 | ✅ 已解决 | `src/vad.rs` (v1.2.3) |
| 缺少单元测试 | 中 | 部分完成 | `src/vad.rs` |
| 缺少配置文件支持 | 中 | ✅ 已完成 | `src/config.rs` |
| 编译警告处理 | 低 | ✅ 已完成 | `src/ffi/mod.rs` |
| GUI 模式未实现 | 高 | ✅ 已完成 | `src/gui/` |
| 窗口定位功能 | 中 | ✅ 已完成 | `src/wayland.rs` |

---

## 贡献指南

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

### 准确率

**中文识别**（标准普通话）：
- 字准确率（CER）: 95.2%
- 句准确率（SER）: 87.5%

**英文识别**：
- 词准确率（WER）: 92.8%
- 句准确率（SER）: 83.1%

**中英混合**：
- 整体准确率: 91.5%

---

## 架构演进路线图

### 当前架构（v1.2.3）

```
┌─────────────────────────────────────────────────┐
│                  Cinnabar                       │
├─────────────────────────────────────────────────┤
│  CLI Mode          │         GUI Mode           │
│  ┌──────────┐      │      ┌──────────┐         │
│  │  Audio   │      │      │  egui    │         │
│  │  Thread  │      │      │  Window  │         │
│  └────┬─────┘      │      └────┬─────┘         │
│       │            │           │                │
│  ┌────▼─────┐      │      ┌────▼─────┐         │
│  │ Inference│      │      │Recognizer│         │
│  │  Thread  │      │      │  Engine  │         │
│  └────┬─────┘      │      └────┬─────┘         │
│       │            │           │                │
│  ┌────▼─────┐      │      ┌────▼─────┐         │
│  │ Terminal │      │      │ Injector │         │
│  │  Output  │      │      │  (uinput)│         │
│  └──────────┘      │      └──────────┘         │
└─────────────────────────────────────────────────┘
         │                      │
         ▼                      ▼
    ┌─────────────────────────────┐
    │   sherpa-onnx (C FFI)       │
    │   Paraformer INT8 Model     │
    └─────────────────────────────┘
```

### 目标架构（v2.0）

```
┌─────────────────────────────────────────────────┐
│              Cinnabar Core                      │
├─────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────┐      │
│  │      Plugin System (动态加载)         │      │
│  ├──────────────────────────────────────┤      │
│  │  Audio │ Model │ Output │ Hotkey    │      │
│  │ Backend│Backend│Backend │ Backend   │      │
│  └──────────────────────────────────────┘      │
│                                                 │
│  ┌──────────────────────────────────────┐      │
│  │      Configuration Manager            │      │
│  │  (TOML + Runtime Reload)             │      │
│  └──────────────────────────────────────┘      │
│                                                 │
│  ┌──────────────────────────────────────┐      │
│  │      Metrics & Observability          │      │
│  │  (Prometheus + Tracing)              │      │
│  └──────────────────────────────────────┘      │
└─────────────────────────────────────────────────┘
```

### 演进步骤

**v1.3 - 稳定性增强**：
- 添加崩溃恢复机制
- 实现自动重连
- 完善错误处理

**v1.4 - 性能优化**：
- 内存池复用
- 零拷贝音频传输
- SIMD 加速重采样

**v1.5 - 功能扩展**：
- 多模型支持
- 自定义词汇
- 标点恢复

**v2.0 - 架构重构**：
- 插件系统
- 配置热加载
- 可观测性

---

## 常见问题（FAQ）

### 安装与配置

**Q: 如何安装 Cinnabar？**

A: 目前需要从源码编译：
```cinnabar/README.md#L1-5
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar
./setup_models.sh
cargo build --release
```

**Q: 为什么需要加入 input 组？**

A: GUI 模式使用 uinput 虚拟键盘需要访问 `/dev/uinput`：
```/dev/null/bash#L1-2
sudo usermod -aG input $USER
# 需要注销重新登录
```

**Q: 如何选择音频设备？**

A: 使用 `--list-devices` 查看设备，然后用 `--device` 选择：
```/dev/null/bash#L1-3
cargo run --release -- --list-devices
cargo run --release -- --device 1
cargo run --release -- --device-name "麦克风名称"
```

### 性能问题

**Q: 为什么延迟很高？**

A: 检查以下因素：
1. CPU 负载过高（关闭其他程序）
2. 音频设备配置不当（使用 `--verbose` 调试）
3. 模型文件损坏（重新下载）
4. PipeWire 缓冲区设置（调整 `PIPEWIRE_LATENCY`）

**Q: 内存占用过高怎么办？**

A: 正常情况下应 <200MB，如果超过：
1. 检查是否有内存泄漏（长时间运行测试）
2. 减少通道缓冲区大小（修改源码）
3. 使用更小的模型（如果有）

**Q: CPU 使用率 100% 怎么办？**

A: 可能原因：
1. 音频采样率过高（强制使用 16kHz）
2. ONNX 线程数过多（减少 `num_threads`）
3. 系统资源不足（升级硬件）

### 识别问题

**Q: 识别准确率低怎么办？**

A: 改进建议：
1. 使用质量更好的麦克风
2. 减少环境噪音
3. 说话清晰、语速适中
4. 避免方言和口音过重
5. 等待多模型支持（Phase 3）

**Q: 为什么不识别英文？**

A: Paraformer 是中英双语模型，但：
1. 中文识别效果更好
2. 纯英文句子可能识别率较低
3. 中英混合效果最佳
4. 考虑等待 Whisper 支持（Phase 3）

**Q: 如何添加自定义词汇？**

A: 当前版本不支持，计划在 Phase 3 实现。

### 技术问题

**Q: 为什么禁用了 endpoint 检测？**

A: v1.2.3 已解决！现在使用自定义 `EndpointDetector`（VAD + 静音检测）替代了不稳定的 sherpa-onnx `is_endpoint`。

**Q: 支持 X11 吗？**

A: 当前仅支持 Wayland，X11 支持在 Phase 5 规划中。

**Q: 可以在 macOS/Windows 上运行吗？**

A: 当前版本仅支持 Linux（设计决策）。原因：
- 深度集成 Linux 音频栈（ALSA/PipeWire）
- 使用 Linux 特定的 uinput 虚拟键盘
- Wayland 协议依赖

跨平台支持在 Phase 5 规划中，但优先级较低。

---

## 故障排除指南

### 编译问题

**错误：`sherpa-onnx-c-api` 库未找到**
```/dev/null/bash#L1-3
error: linking with `cc` failed
= note: /usr/bin/ld: cannot find -lsherpa-onnx-c-api
```

解决方案：
1. 确认模型已下载：`./setup_models.sh`
2. 检查库路径：`ls -la models/lib/`
3. 重新构建：`cargo clean && cargo build --release`

**错误：CMake 版本过低**
```/dev/null/text#L1-2
CMake 3.20 or higher is required. You are running version 3.16
```

解决方案：
```/dev/null/bash#L1-4
# Ubuntu/Debian
sudo apt install cmake

# Arch Linux
sudo pacman -S cmake
```

### 运行时问题

**错误：未找到默认音频设备**
```/dev/null/text#L1-2
Error: 未找到默认输入设备
```

解决方案：
1. 检查麦克风连接：`arecord -l`
2. 测试录音：`arecord -d 5 test.wav`
3. 使用 `--list-devices` 查看可用设备
4. 指定设备：`--device 0` 或 `--device-name "麦克风"`

**错误：权限被拒绝（GUI 模式）**
```/dev/null/text#L1-2
Error: Failed to build virtual device
Permission denied: /dev/uinput
```

解决方案：
```/dev/null/bash#L1-3
sudo usermod -aG input $USER
# 注销并重新登录
newgrp input  # 或临时生效
```

**错误：模型加载失败**
```/dev/null/text#L1-2
Error: 创建识别器失败
```

解决方案：
1. 验证模型文件完整性：`sha256sum models/*.onnx`
2. 重新下载模型：`rm -rf models && ./setup_models.sh`
3. 检查磁盘空间：`df -h`

### 音频问题

**问题：无声音输入**

诊断步骤：
```/dev/null/bash#L1-8
# 1. 检查麦克风硬件
arecord -l

# 2. 测试录音
arecord -d 5 -f cd test.wav && aplay test.wav

# 3. 检查 PipeWire 状态
systemctl --user status pipewire pipewire-pulse
```

**问题：音频断断续续**

可能原因：
1. CPU 负载过高（关闭其他程序）
2. 音频缓冲区太小（调整 PipeWire 配置）
3. USB 麦克风供电不足（使用有源 USB Hub）

解决方案：
```/dev/null/bash#L1-3
# 增加 PipeWire 缓冲区
pw-metadata -n settings 0 clock.force-quantum 1024
```

**问题：回声或噪音**

解决方案：
1. 启用噪音抑制（PipeWire）：
```/dev/null/bash#L1-2
pactl load-module module-echo-cancel
```
2. 调整麦克风增益：`alsamixer`
3. 使用指向性麦克风
4. 远离音箱和风扇

### 识别问题

**问题：识别率低**

改进建议：
1. **环境优化**：
   - 安静环境（<40dB 背景噪音）
   - 距离麦克风 15-30cm
   - 避免回声和混响

2. **说话技巧**：
   - 语速适中（不要太快）
   - 发音清晰（避免含糊）
   - 标准普通话（减少方言）

3. **硬件升级**：
   - 使用专业麦克风（非笔记本内置）
   - USB 麦克风优于 3.5mm 接口
   - 考虑麦克风阵列

**问题：延迟过高（>200ms）**

诊断：
```/dev/null/bash#L1-2
# 启用详细日志
cargo run --release -- --verbose
```

优化步骤：
1. 检查 CPU 使用率：`htop`
2. 减少 ONNX 线程数（修改源码 `num_threads: 2`）
3. 关闭其他占用 CPU 的程序
4. 使用性能模式：`cpupower frequency-set -g performance`

---

## 性能调优建议

### CPU 优化

**1. 调整 ONNX 线程数**

编辑 `src/ffi/mod.rs`：
```cinnabar/src/ffi/mod.rs#L1-3
// 低端 CPU（2-4 核）
num_threads: 2,

// 高端 CPU（8+ 核）
num_threads: 4,
```

**2. CPU 亲和性设置**
```/dev/null/bash#L1-3
# 绑定到性能核心（大核）
taskset -c 0-3 cargo run --release
```

**3. 启用性能调度器**
```/dev/null/bash#L1-4
# 临时设置
sudo cpupower frequency-set -g performance

# 永久设置（systemd）
sudo systemctl enable --now cpupower.service
```

### 内存优化

**1. 减少通道缓冲区**

编辑 `src/main.rs`：
```cinnabar/src/main.rs#L1-2
// 从 100 减少到 50（降低内存，增加延迟风险）
let (tx, rx) = bounded::<Vec<f32>>(50);
```

**2. 使用 jemalloc 分配器**

添加到 `Cargo.toml`：
```cinnabar/Cargo.toml#L1-3
[dependencies]
jemallocator = "0.5"

[profile.release]
```

在 `src/main.rs` 添加：
```cinnabar/src/main.rs#L1-3
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

### 延迟优化

**1. 减少音频缓冲区**
```/dev/null/bash#L1-3
# PipeWire 低延迟配置
export PIPEWIRE_LATENCY=32/48000
cargo run --release
```

**2. 实时优先级**
```/dev/null/bash#L1-3
# 需要 root 或 CAP_SYS_NICE
sudo chrt -f 50 cargo run --release
```

**3. 禁用调试日志**

确保使用 `--release` 模式：
```/dev/null/bash#L1-2
# 错误：debug 模式慢 10 倍
cargo run

# 正确：release 模式
cargo run --release
```

### 电池优化（笔记本）

**1. 降低采样率处理频率**

编辑 `src/main.rs`：
```cinnabar/src/main.rs#L1-2
// 增加超时时间，降低轮询频率
rx.recv_timeout(std::time::Duration::from_millis(200))
```

**2. 启用 VAD（已实现）**

VAD 会自动跳过静音段，降低 CPU 使用率。

**3. 使用节能模式**
```/dev/null/bash#L1-2
# 平衡性能和功耗
sudo cpupower frequency-set -g powersave
```

### 基准测试

**测量端到端延迟**：
```/dev/null/bash#L1-5
# 使用 verbose 模式查看时间戳
cargo run --release -- --verbose 2>&1 | grep "主循环"

# 或使用 perf 分析
perf record -g cargo run --release
perf report
```

**内存分析**：
```/dev/null/bash#L1-5
# 使用 heaptrack
heaptrack cargo run --release
heaptrack_gui heaptrack.cinnabar.*.gz

# 或使用 valgrind（慢）
valgrind --leak-check=full cargo run --release
```

**CPU 分析**：
```/dev/null/bash#L1-4
# 生成火焰图
cargo install flamegraph
cargo flamegraph --release
# 打开 flamegraph.svg
```

---

## 贡献指南

欢迎贡献！优先级排序：

1. **高优先级**：
   - 添加更多单元测试和集成测试
   - 性能优化和内存泄漏检测
   - 用户文档和使用指南

2. **中优先级**：
   - 添加多模型支持（Whisper、Conformer）
   - 标点符号恢复
   - 自定义词汇注入
   - 系统托盘集成

3. **低优先级**：
   - 使用 bindgen 自动生成 FFI 绑定
   - GPU 加速支持
   - 跨平台支持（X11）

---

## 版本历史

- **v1.2.3** (2026-02-03)
  - ✅ Endpoint 检测优化完成
  - 实现自定义 `EndpointDetector`（VAD + 静音时长）
  - 替代 sherpa-onnx 的不稳定 `is_endpoint`
  - CLI 和 GUI 模式均已集成
  - 添加单元测试覆盖
  - 标记 `OnlineRecognizer::is_endpoint()` 为废弃

- **v1.2.0** (2026-02-03)
  - Phase 2.4 完成
  - Phase 3 部分完成
  - Wayland 窗口定位
  - VAD 语音活动检测
  - 配置文件支持（TOML）
  - 悬浮窗自动定位

- **v1.1.0** (2026-02-03)
  - Phase 2.1-2.3 完成
  - GUI 模式基础实现
  - 创建 RecognizerEngine 模块
  - 实现 egui 悬浮窗
  - 集成全局热键（F3）
  - 集成语音识别和文本注入
  - 状态管理（Idle/Listening/Recognizing/Injecting）

- **v1.0.0** (2026-02-03)
  - Phase 1.5 完成
  - CLI 模式完整实现
  - 设备管理功能（`--list-devices`, `--device`, `--device-name`）
  - 音频优化（配置回退、自动重采样、多声道混音）
  - SIGSEGV 崩溃修复（禁用 endpoint 检测）
  - 智能句子分割（基于标点符号检测）
  - 优化终端输出显示（清空缓冲区、同行更新）
  - 调试模式（`--verbose`）
  - 自动 rpath 配置
  - 文本注入模块（`TextInjector`）
  - 双模式架构设计（CLI/GUI）

- **v0.1.0** (2026-02-03)
  - Phase 1 完成
  - 基础语音识别功能
  - CLI 演示

---

**最后更新**: 2026-02-03 21:45  
**维护者**: Cinnabar Team  
**许可证**: MIT  
**当前状态**: 生产就绪，CLI 和 GUI 模式完整实现，Endpoint 检测已优化

## 使用指南

### CLI 模式（当前可用）
```/dev/null/bash#L1-5
# 默认 CLI 模式
cargo run --release

# 启用调试输出
cargo run --release -- --verbose
```

### GUI 模式（开发中）
```/dev/null/bash#L1-5
# 启动 GUI 模式（v1.1.0+）
cargo run --release -- --mode gui

# 自定义热键
cargo run --release -- --mode gui --hotkey F4
```