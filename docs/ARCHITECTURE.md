# 流式 + VAD + 非流式 ASR 架构设计

本文档解释 `cinnabar` 当前 ASR pipeline 的架构、数据流与时序关系。
面向想理解"为什么 partial 和 final 文本一致"、"为什么 VAD 切段后要等 200ms"、
"非流式 ASR 何时被调用"等问题的开发者。

---

## 1. 三大组件的角色

| 组件 | 角色 | 运行时机 | 输出 |
|---|---|---|---|
| **流式 ASR**（Paraformer streaming-zh-en） | 实时反馈，跟手显示 | **常驻**，每 ~41ms 音频帧跑一次 | partial（增量文本）+ final（切段时刻的快照） |
| **VAD**（ten-vad） | 句子边界检测 | **常驻**，每帧跑一次 | speech 段（含起止 1.2s 静音尾） |
| **非流式 ASR**（Zipformer CTC 中文 int8） | 准确定稿 | **按需**，每个 VAD 段一次 | 精修文本 |

三者**完全独立运行**，只在 VAD 切段那一刻发生交汇。

类比：
- **流式 ASR** ≈ "自言自语复述"——边听边小声复述（"今...今天...今天天气"）
- **VAD** ≈ "句子边界判断"——听到静音期就提醒"该整理了"
- **非流式 ASR** ≈ "听完后整理"——完整听完一段后给出最终版（"今天天气好啊"）

---

## 2. 数据流（一个 VAD 段的生命周期）

```
                    ┌──────────────────────────────────────────┐
麦克风 ── cpal ────►│  audio callback (Vec<f32>)               │
                    │  每 ~41ms 一次（按设备 SR）              │
                    └─────────┬───────────────┬────────────────┘
                              │               │
                              ▼               ▼
                    ┌──────────────────┐    ┌──────────────────┐
                    │ 流式 OnlineRecog │    │ ten-vad VAD       │
                    │ - accept 每帧    │    │ - accept 每帧      │
                    │ - decode while   │    │ - 持续跟踪语音/静音│
                    │   ready          │    │ - min_silence=1.2s│
                    │ - get_result     │    │   触发 endpoint   │
                    │   → partial      │    │ - 输出 SpeechSeg  │
                    │                  │    │   {start, samples}│
                    └────────┬─────────┘    └────────┬───────────┘
                             │                       │
                             │ partial                │ segments
                             ▼                       ▼
                    ┌──────────────────┐    ┌──────────────────┐
                    │ print_partial    │    │ drain_segments   │
                    │ (节流 150ms)     │    │ → Vec<SpeechSeg> │
                    │ ▌ 今天天气好     │    │                  │
                    └──────────────────┘    └────────┬─────────┘
                                                      │
                                                      ▼  (每个 seg)
                                            ┌────────────────────┐
                                            │ 路径 A（流式 final）│
                                            │ recognizer.get_result│
                                            │ → 流式当时输出      │
                                            │ → print_final ✅     │
                                            │ → log final         │
                                            └────────┬───────────┘
                                                     │
                                                     ▼  (200ms 后)
                                            ┌────────────────────┐
                                            │ 路径 B（非流式     │
                                            │ refine）             │
                                            │ OfflineRecognizer   │
                                            │ accept→decode→get  │
                                            │ → print_final_     │
                                            │   replace（覆盖）   │
                                            │ → combined_score    │
                                            │ → 决策 override /   │
                                            │   warn / rejected   │
                                            │ → log refine         │
                                            └────────────────────┘
```

---

## 3. 时序图

```
时间(s)        0.0    0.5    1.0    1.5    2.0    2.5    3.0    3.5    4.0    4.5
               |      |      |      |      |      |      |      |      |      |

音频帧         F1 F2 F3 F4 F5 F6 F7 F8 F9 ... F~50 Fs Fs Fs ...
               (每帧 ~41ms, 16kHz, 640 样本)
               ↑
               cpal 回调每 ~41ms 调一次

VAD 状态       [═══════ speech ═══════][▒▒▒▒▒▒▒▒▒▒▒▒▒▒][═════ next ═════]
                                       ↑
                                       │
                                       min_silence (1.2s) 触发
                                       drain_segments → seg
                                       seg.samples = 语音 + 1.2s 静音

stream         ▌今   ▌今天    ▌今天天    ▌今天天气好    ▌今天天气好
partials       (节流 150ms · 每帧 partial 仅一次 printed=true 事件)
                                         ↑
                                         流式已稳定
                                         (但 VAD 还没切段)

               ╔════════════════════════════╗                 ╔════════════════╗
stream         ║ ✅ 今天天气好 (流式 final)    ║                 ║ ✅ 出去玩的人很多 ║
final          ║ 路径 A: get_result()          ║                 ║ 路径 A: 同上      ║
(立即)         ║ = VAD 切段时流的最新 partial   ║                 ║                    ║
               ╚══════════════════╤═══════════╝                 ╚═══════════════════╝
                                  │ ~0ms                                    │ ~0ms
                                  ▼                                         ▼
                          屏幕显示 ✅                              屏幕显示 ✅

                                  │ ~200ms                                   │ ~200ms
                                  ▼                                         ▼
               ╔════════════════════════════╗                 ╔════════════════╗
offline        ║ ↻ refine                    ║                 ║ ↻ refine        ║
refine         ║ 路径 B: OfflineRecognizer   ║                 ║ 路径 B: 同上    ║
(覆盖)         ║ accept → decode → get      ║                 ║                  ║
               ║                            ║                 ║                  ║
               ║ combined_score(stream,      ║                 ║ combined_score    ║
               ║   refined) → 决策          ║                 ║   → 决策         ║
               ║                            ║                 ║                  ║
               ║  ≥ 0.85 → override         ║                 ║  ≥ 0.85 → override
               ║  ≥ 0.40 → override_warn    ║                 ║                  ║
               ║  < 0.40 → rejected         ║                 ║                  ║
               ╚════════════════════════════╝                 ╚════════════════╝
                                  │                                         │
                                  ▼ (if override / warn)                     ▼
                          屏幕覆盖上一行 ✅                          屏幕覆盖 ✅

JSONL          partial seq=1 "今"          printed=false        refine events:
事件流         partial seq=1 "今"          printed=true         ├ streaming="..."
               partial seq=2 "今天"        printed=false        ├ refined="..."
               partial seq=3 "今天天"      printed=false        ├ score=0.85
               partial seq=4 "今天天气好"   printed=false        └ decision=override
               partial seq=4 "今天天气好"   printed=true
               endpoint seg=0
               final "今天天气好"
               refine (200ms 后)
               ──── 下一段 ────
```

---

## 4. 三个常被误解的关键点

### 4.1 流式 ASR 的 final **不做任何"总结"**

```cpp
// sherpa-onnx 1.12.9 内部实现（简化）
const char* SherpaOnnxGetOnlineStreamResult(recognizer, stream) {
    return stream->result.text.c_str();  // 就是 getter
}
```

它就是**流式 ASR 当前帧最新 partial 的快照**。没有：
- 后处理（标点 / ITN / 大小写）
- 重新打分（不像 batch 模式跑 beam search）
- 上下文融合
- 长度归一化

JSONL 里看到的**最后一条 partial 文本**与 **final 事件文本**完全相同。

### 4.2 流式 partial 比 VAD 切段早 ~0.5-1.0s 就稳定了

```
▌今 ▌今天 ▌今天天 ▌今天天气好 ▌今天天气好 ▌今天天气好
───── 0.0s ───── 0.5s ──── 1.0s ──── 1.5s ──── 2.0s
                       ↑ 流式稳定（但 VAD 还在等 1.2s 静音）
                                            ↑
                                       VAD 在 2.6s 切段
                                       （用户停了 1.2s 之后）
```

**流式 ASR 早已"知道答案"，VAD 还在等静音阈值**。这就是为什么流式 ASR 的 partial 看起来"跟手"——VAD 慢一拍切段时，流式早就稳定了。

### 4.3 流式 final 的"质量"取决于切段时机

流式解码是带状态的过程。同一个音频：
- 在 T+2.5s 切段时可能输出 "今天天气好"
- 在 T+2.6s 切段时可能输出 "今天天气好啊"
- 在 T+2.8s 切段时可能输出 "今天天气好"

晚 100ms 切段，结果可能变好也可能变差（取决于解码器 lattice 状态）。
VAD 用 1.2s 静音阈值保证"流式已经稳定了才切"，降低采样随机性。

---

## 5. 三档决策（combined_score）

非流式 refine 出结果后，对比流式 final 的文本相似度，决定是否覆盖屏幕：

```
score = char_jaccard × length_ratio × prefix_match

char_jaccard   字符级 multiset Jaccard    （"用了多少相同字"）
length_ratio   min/max 字符数             （惩罚"精修显著短"）
prefix_match   前 5 字位置匹配率          （惩罚"丢前缀"型劣化）

阈值：
  score ≥ 0.85  → override      （信任精修，覆盖屏幕）
  score ≥ 0.40  → override_warn （覆盖但日志标 warn）
  score < 0.40  → rejected      （不覆盖，保留流式）
```

**为什么 prefix_match 是关键**：Zipformer CTC 中文模型的系统性问题偏向"丢前缀"，
例如：
- "然后会议的页面..." → "的页面..."（丢"然后会议"）
- "接下来..." → "还有..."（丢"接下来"）

字符级 Jaccard 看"用了多少相同字"对这些变化不敏感（前缀短的字集重合度低，但
Jaccard 是 `set/set`），prefix_match 用对齐位置命中率抓出"前 5 字对不上"，
combined_score 直接掉到 0。

---

## 6. 流式 final 与非流式 final 的对比

| 维度 | 流式 final | 非流式 final |
|---|---|---|
| **音频量** | 切段时刻累积的所有音频 | 同样那段音频 |
| **解码算法** | 在线贪心（CTC greedy + transducer） | 离线 CTC greedy |
| **上下文利用** | 只利用了过去 | 完整利用（前后都能看） |
| **后处理** | 无 | 无 |
| **计算量** | 边际（每个新帧都跑过） | 一次性 ~150-300ms |
| **CER 量级** | 6-10% | 2-5% |

主要差异在**上下文利用**：流式只能基于"已经收到的音频"贪心选 token；
非流式可以看完整段、做更平衡的取舍。

---

## 7. 关键 invariant

1. **流式 partial 比 VAD 切段早稳定** —— 流式 ASR 跟手感来自这个 invariant
2. **流式 final = 最后一条 partial 文本** —— 没有"事后总结"
3. **VAD 与流式 ASR 独立** —— VAD 不知道流式 partial 的内容，反之亦然
4. **路径 A 与路径 B 互不干扰** —— 流式 final 立即显示，refine 200ms 后决定是否覆盖
5. **三档决策只看文本相似度** —— 不依赖 ASR 原生置信度（sherpa-onnx 1.12.9 没暴露）

---

## 8. 典型错误模式

| 错误 | 流式 vs 非流式 | 备注 |
|---|---|---|
| 同音字混淆（"在/再"） | 流式更易错 | 贪心无回溯 |
| 末尾截断 | 两者都可能 | 流式等下一个静音期才"提交" |
| 短句段边界误判 | VAD 决定 | 短停顿可能被 ten-vad 切开 |
| 长段流式串扰 | 仅流式 | 长段 attention 容易复读前面 |
| 精修丢技术术语 | 仅非流式 | CTC 偏短，丢细节 |