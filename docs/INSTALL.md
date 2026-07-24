# Cinnabar 安装指南

本文档提供 Cinnabar 在不同 Linux 发行版上的详细安装步骤。

## 目录

- [系统要求](#系统要求)
- [快速安装](#快速安装)
- [分发行版安装](#分发行版安装)
- [从源码构建](#从源码构建)
- [权限配置](#权限配置)
- [验证安装](#验证安装)
- [故障排除](#故障排除)

---

## 系统要求

### 硬件要求

**最低配置**：
- CPU: 双核 x86_64 处理器
- RAM: 2GB
- 存储: 100MB 可用空间
- 音频: 任意麦克风设备

**推荐配置**：
- CPU: 四核及以上 x86_64 处理器
- RAM: 4GB 及以上
- 存储: 500MB 可用空间（包含多个模型）
- 音频: USB 麦克风或专业音频接口

### 软件要求

**必需**：
- Linux 发行版（Kernel 5.10+）
- Rust 1.70+ (2021 edition)
- CMake 3.20+
- Git

**音频栈**（任选其一）：
- PipeWire 0.3+ （推荐）
- PulseAudio 15.0+
- ALSA 1.2+

**GUI 模式额外要求**：
- Wayland 合成器（Sway, Hyprland, GNOME, KDE Plasma）
- 或 X11 显示服务器（实验性支持）

---

## 快速安装

### 一键安装脚本

```cinnabar/docs/INSTALL.md#L1-10
# 下载并运行安装脚本
curl -fsSL https://raw.githubusercontent.com/yourusername/cinnabar/main/install.sh | bash

# 或使用 wget
wget -qO- https://raw.githubusercontent.com/yourusername/cinnabar/main/install.sh | bash
```

### 手动安装（推荐）

```cinnabar/docs/INSTALL.md#L1-15
# 1. 克隆仓库
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar

# 2. 下载模型
./setup_models.sh

# 3. 编译安装
cargo build --release

# 4. 验证安装
cargo run --release -- --list-devices
```

---

## 分发行版安装

### Arch Linux / Manjaro

```cinnabar/docs/INSTALL.md#L1-15
# 安装依赖
sudo pacman -S rust cmake git base-devel alsa-lib pipewire

# 克隆仓库
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar

# 下载模型
./setup_models.sh

# 编译安装
cargo build --release

# 可选：安装到系统路径
sudo install -Dm755 target/release/cinnabar /usr/local/bin/cinnabar
sudo mkdir -p /usr/local/share/cinnabar
sudo cp -r models /usr/local/share/cinnabar/
```

### Ubuntu / Debian

```cinnabar/docs/INSTALL.md#L1-20
# 更新包列表
sudo apt update

# 安装依赖
sudo apt install -y curl git build-essential cmake pkg-config \
    libasound2-dev libpipewire-0.3-dev

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 克隆仓库
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar

# 下载模型
./setup_models.sh

# 编译安装
cargo build --release
```

### Fedora / RHEL

```cinnabar/docs/INSTALL.md#L1-15
# 安装依赖
sudo dnf install -y rust cargo cmake git gcc-c++ \
    alsa-lib-devel pipewire-devel

# 克隆仓库
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar

# 下载模型
./setup_models.sh

# 编译安装
cargo build --release
```

---

## 从源码构建

### 详细构建步骤

**1. 安装 Rust 工具链**

```cinnabar/docs/INSTALL.md#L1-10
# 使用 rustup 安装
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 配置环境变量
source $HOME/.cargo/env

# 验证安装
rustc --version
cargo --version
```

**2. 安装系统依赖**

根据您的发行版选择对应的命令（见上文）。

**3. 克隆仓库**

```cinnabar/docs/INSTALL.md#L1-5
git clone https://github.com/yourusername/cinnabar.git
cd cinnabar

# 或使用 SSH
git clone git@github.com:yourusername/cinnabar.git
```

**4. 下载模型文件**

```bash
# 运行模型下载脚本（同时下载 ASR + ten-vad）
./setup_models.sh

# 验证模型文件
ls -lh models/
# 应该看到：
# - encoder.int8.onnx (~20MB)        ← ASR
# - decoder.int8.onnx (~20MB)        ← ASR
# - tokens.txt     (~500KB)          ← ASR
# - ten-vad.onnx  (~324KB)           ← VAD（v1.2.4+）
```

如果 ten-vad 模型缺失，会在启动时报错 `创建 ten-vad VoiceActivityDetector 失败`；可手动下载：

```bash
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/ten-vad.onnx \
     -O models/ten-vad.onnx
```

**5. 编译项目**

```cinnabar/docs/INSTALL.md#L1-15
# Release 模式（推荐）
cargo build --release

# Debug 模式（开发用）
cargo build

# 编译时间：首次约 5-10 分钟
# 二进制文件位置：target/release/cinnabar
```

**6. 运行测试**

```cinnabar/docs/INSTALL.md#L1-5
# 运行单元测试
cargo test --release

# 运行 CLI 模式
cargo run --release
```

---

## 权限配置

### GUI 模式权限（uinput）

GUI 模式需要访问 `/dev/uinput` 来模拟键盘输入。

**方案 1：添加用户到 input 组（推荐）**

```cinnabar/docs/INSTALL.md#L1-10
# 添加当前用户到 input 组
sudo usermod -aG input $USER

# 注销并重新登录生效
# 或临时生效（当前终端）
newgrp input

# 验证权限
ls -l /dev/uinput
```

**方案 2：创建 udev 规则**

```cinnabar/docs/INSTALL.md#L1-10
# 创建 udev 规则文件
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | \
    sudo tee /etc/udev/rules.d/99-cinnabar.rules

# 重新加载 udev 规则
sudo udevadm control --reload-rules
sudo udevadm trigger

# 验证
ls -l /dev/uinput
```

### 音频设备权限

```cinnabar/docs/INSTALL.md#L1-5
# 添加用户到 audio 组（某些发行版需要）
sudo usermod -aG audio $USER

# 注销并重新登录
```

---

## 验证安装

### 基本功能测试

**1. 检查二进制文件**

```cinnabar/docs/INSTALL.md#L1-5
# 查看版本信息
./target/release/cinnabar --version

# 查看帮助信息
./target/release/cinnabar --help
```

**2. 列出音频设备**

```cinnabar/docs/INSTALL.md#L1-5
# 列出所有可用的音频输入设备
cargo run --release -- --list-devices

# 应该看到类似输出：
# 可用的音频输入设备：
#   [0] 默认麦克风 - 48000 Hz, 2 声道
#   [1] USB 麦克风 - 44100 Hz, 1 声道
```

**3. 测试 CLI 模式**

```cinnabar/docs/INSTALL.md#L1-10
# 启动 CLI 模式
cargo run --release

# 说话测试
# 应该看到实时识别结果

# 按 Ctrl+C 退出
```

**4. 测试 GUI 模式**

```cinnabar/docs/INSTALL.md#L1-10
# 启动 GUI 模式
cargo run --release -- --mode gui

# 按 F3 开始/停止录音
# 说话后应该自动注入文本到当前应用

# 关闭悬浮窗退出
```

### 性能测试

```cinnabar/docs/INSTALL.md#L1-10
# 启用详细日志查看性能
cargo run --release -- --verbose

# 观察延迟指标
# 正常情况下应该 <100ms
```

---

## 故障排除

### 常见问题

**问题 1：找不到 libsherpa-onnx-c-api.so**

```cinnabar/docs/INSTALL.md#L1-5
# 解决方案：使用 cargo run（自动设置 rpath）
cargo run --release

# 或手动设置 LD_LIBRARY_PATH
export LD_LIBRARY_PATH=$PWD/models/lib:$LD_LIBRARY_PATH
```

**问题 2：编译失败 - CMake 版本过低**

```cinnabar/docs/INSTALL.md#L1-5
# 升级 CMake
# Ubuntu/Debian
sudo apt install cmake

# Arch Linux
sudo pacman -S cmake
```

**问题 3：权限被拒绝（/dev/uinput）**

```cinnabar/docs/INSTALL.md#L1-3
# 参考上文"权限配置"章节
sudo usermod -aG input $USER
```

**问题 4：未找到音频设备**

```cinnabar/docs/INSTALL.md#L1-10
# 检查 PipeWire 状态
systemctl --user status pipewire pipewire-pulse

# 重启 PipeWire
systemctl --user restart pipewire pipewire-pulse

# 测试麦克风
arecord -d 5 test.wav && aplay test.wav
```

### 获取更多帮助

如果遇到其他问题，请查看：

- [故障排除指南](TROUBLESHOOTING.md) - 详细的问题诊断和解决方案
- [性能调优指南](PERFORMANCE.md) - 性能优化建议
- [开发路线图](ROADMAP.md) - 项目规划和 FAQ
- [GitHub Issues](https://github.com/yourusername/cinnabar/issues) - 提交问题

---

## 卸载

### 完全卸载

```cinnabar/docs/INSTALL.md#L1-15
# 删除二进制文件
sudo rm -f /usr/local/bin/cinnabar

# 删除模型文件
sudo rm -rf /usr/local/share/cinnabar

# 删除源码目录
cd ..
rm -rf cinnabar

# 删除 udev 规则（如果创建了）
sudo rm -f /etc/udev/rules.d/99-cinnabar.rules
sudo udevadm control --reload-rules

# 从用户组移除（可选）
sudo gpasswd -d $USER input
```

---

**最后更新**：2026-02-03  
**版本**：v1.2.3