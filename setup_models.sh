#!/bin/bash

# 下载 Cinnabar 需要的全部模型：
# - ASR 流式（Zipformer transducer, 中英双语 + 内置标点, ~127MB）
# - ASR 非流式（Zipformer CTC 中文 int8，~301MB）— 用于切段后精修
# - VAD（ten-vad）
# 每个文件按存在性判断是否下载 —— `./models` 目录保留，不会被整体删除。

set -e

# 流式 Zipformer transducer（127MB, 中英双语 + 内置标点）
ASR_MODEL_NAME="sherpa-onnx-x-asr-960ms-streaming-zipformer-transducer-zh-en-punct-int8-2026-06-05"
ASR_MODEL_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${ASR_MODEL_NAME}.tar.bz2"

# 非流式精修模型：Zipformer CTC 中文 int8（中文场景 AISHELL WER 1.74%）
# 不支持中英双语，若需要英文请改用 paraformer-zh-small 或 sense_voice。
# 非流式精修模型：Paraformer zh-2023-09-14（中文 attention decoder）
# 选 Paraformer 而非 Zipformer CTC / SenseVoice：
#   - attention decoder 在 beam search 阶段对热词加权（CTC 框架弱，SenseVoice 不支持）
#   - 不会过度摘要（与之前用的 Zipformer CTC 不同）
OFFLINE_MODEL_NAME="sherpa-onnx-paraformer-zh-2023-09-14"
OFFLINE_MODEL_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${OFFLINE_MODEL_NAME}.tar.bz2"

VAD_MODEL_NAME="ten-vad.onnx"
VAD_MODEL_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${VAD_MODEL_NAME}"

MODEL_DIR="./models"
TMP_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

echo "🔥 Cinnabar Model Setup"
echo "Target directory: ${MODEL_DIR}"
echo ""

mkdir -p "${MODEL_DIR}"

# ---- ASR 流式模型（Zipformer transducer: encoder/decoder/joiner + tokens）----
ASR_DIR="${MODEL_DIR}/${ASR_MODEL_NAME}"
need_asr=0
for f in encoder.int8.onnx decoder.onnx joiner.int8.onnx tokens.txt; do
    if [ ! -f "${ASR_DIR}/${f}" ]; then
        need_asr=1
        break
    fi
done

if [ "${need_asr}" -eq 1 ]; then
    echo "📥 Downloading ASR streaming model: ${ASR_MODEL_NAME} (~127MB)..."
    wget -q --show-progress "${ASR_MODEL_URL}" -O "${TMP_DIR}/model.tar.bz2"
    echo "📦 Extracting ASR model..."
    tar -xjf "${TMP_DIR}/model.tar.bz2" -C "${TMP_DIR}/"
    mkdir -p "${ASR_DIR}"
    # 拷贝 tarball 顶层所有 .onnx + tokens.txt 到子目录
    cp -n "${TMP_DIR}/${ASR_MODEL_NAME}"/*.onnx "${ASR_DIR}/" 2>/dev/null || true
    cp -n "${TMP_DIR}/${ASR_MODEL_NAME}/tokens.txt" "${ASR_DIR}/" 2>/dev/null || true
    echo "✅ ASR streaming model installed."
else
    echo "✅ ASR streaming model already present, skipping."
fi

# ---- ASR 非流式（精修用）模型 ----
OFFLINE_DIR="${MODEL_DIR}/${OFFLINE_MODEL_NAME}"
if [ ! -f "${OFFLINE_DIR}/model.int8.onnx" ] || [ ! -f "${OFFLINE_DIR}/tokens.txt" ]; then
    echo "📥 Downloading ASR offline model: ${OFFLINE_MODEL_NAME} (~223MB)..."
    wget -q --show-progress "${OFFLINE_MODEL_URL}" -O "${TMP_DIR}/offline.tar.bz2"
    echo "📦 Extracting offline model..."
    mkdir -p "${OFFLINE_DIR}"
    tar -xjf "${TMP_DIR}/offline.tar.bz2" -C "${TMP_DIR}/"
    cp "${TMP_DIR}/${OFFLINE_MODEL_NAME}"/*.onnx "${OFFLINE_DIR}/" 2>/dev/null || true
    cp "${TMP_DIR}/${OFFLINE_MODEL_NAME}/tokens.txt" "${OFFLINE_DIR}/" 2>/dev/null || true
    echo "✅ ASR offline model installed."
else
    echo "✅ ASR offline model already present, skipping."
fi

# ---- ten-vad 模型 ----
if [ ! -f "${MODEL_DIR}/${VAD_MODEL_NAME}" ]; then
    echo "📥 Downloading ten-vad model..."
    wget -q --show-progress "${VAD_MODEL_URL}" -O "${MODEL_DIR}/${VAD_MODEL_NAME}"
    echo "✅ ten-vad model installed."
else
    echo "✅ ten-vad model already present, skipping."
fi

echo ""
echo "📂 Models in ${MODEL_DIR}:"
ls -lh "${MODEL_DIR}"
echo ""
ls -lh "${OFFLINE_DIR}" 2>/dev/null || true

echo ""
echo "🚀 Run: cargo run --release"