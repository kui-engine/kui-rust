# DevTools 性能面字段契约

> 本文件回应 consumer 审查单 `ASK-KUI-3.57.0-R2` 的 G3 / G4，以及 `ASK-KUI-3.55.0-R1`
> 欠下的 D5。目标是让 consumer 做性能门禁时**不必靠试探**：字段叫什么、分母是什么、
> 什么时候为 0、哪个是派生值哪个是引擎值，都写在这里。
>
> - 生效版本：**3.58.0**
> - schema：`r29.recording.v2`（以 `Performance.getCapabilities` 的 `schemaVersion` 为准）

---

## 1. 先澄清一个术语：`repaintAreaPercent` 不是 CDP 字段

这是 R1-C6 答复里没说清、导致 consumer 白找一轮的地方，先纠正。

`repaintAreaPercent` **从来不存在于 CDP 协议面**。它是 KUI 仓内 adv 测试框架
`examples/adv_examples/tools/advex_run.py` 的**派生指标**：

```python
# advex_run.py（约 L316-324）
rr = cdp.send("Performance.repaintRects", {"sinceFrame": perf_baseline})
vp_area = float(vp["width"]) * float(vp["height"])     # driver.json 声明的 viewport
max_area = 0.0
for fr in rr.get("frames", []):
    if fr.get("overflow"):
        a = vp_area                                     # 矩形数溢出记录上限 → 记为全屏
    else:
        a = sum(rc["w"] * rc["h"] for rc in fr.get("rects", []))
    max_area = max(max_area, a)
perf["repaintAreaPercent"] = round(100.0 * max_area / vp_area, 1)
```

要点：

- 取的是**驱动期间单帧最大重绘面积**，不是平均；
- 分母是 **driver.json 里声明的 viewport**，不是引擎自报的任何量；
- `overflow=true`（脏矩形条数超过记录上限）会被**整帧记为 100%**——所以这个指标偏保守，
  它回答的是「最坏一帧至多重绘了多大比例」，不是「实际画了多少」。

**R1-C6 让 consumer「在新包上重采 `repaintAreaPercent`」，正确的读法是「用 adv 框架重跑，
看这个派生值」，不是「去 CDP 找这个字段」。** 这句话我方没写清楚，致歉。

## 2. 引擎侧的对应物

| 你想知道的 | 去哪拿 | 说明 |
|---|---|---|
| 单帧脏区占视口比例 | `Performance.getRecording` → `frames[].render.damagePercentViewport` | 引擎自算，分母见下 |
| 脏区绝对面积 | `frames[].render.damageAreaCssPx` | CSS 像素² |
| 脏矩形条数 | `frames[].render.damageRectCount` | 与 `repaintRects` 的 `rects` 条数同源 |
| 分母 | `frames[].render.viewportAreaCssPx` | **后端 surface 尺寸 ÷ devicePixelRatio** |
| 绘制被脏区裁掉多少 | `frames[].work.paintCullDamage` | 被 damage 判据跳过的 paint visit 数 |
| 原始脏矩形 | `Performance.repaintRects` | 逐帧 `{i, mode, overflow, rects[]}` |

**分母的一个已知不一致，必须说明**：`viewportAreaCssPx` 取的是**后端 surface ÷ dpr**，
而绘制侧的两条裁剪判据用的是**布局视口**（`painter.cpp` 的 `rootPaintWidth_` /
`rootPaintHeight_`）。两者在滚动条预留、DPR 取整、窗口边框上都可能不等。
3.56.0 起 `Performance.renderStats` 的 `layoutViewport` 块把布局视口、surface 的 CSS
像素尺寸、dpr 和根/body 滚动偏移一并落盘，需要精确对账时请用它，不要假设两者相同。

## 3. `Performance.renderStats` 的 summary 键

共 16 个，按语义分组：

| 键 | 含义 | 何时为 0 / 退化 |
|---|---|---|
| `framesPainted` | 真正走了绘制的帧数 | — |
| `framesReused` | 复用上一帧结果、跳过绘制的帧数 | retained DL 关闭时恒 0 |
| `skipRate` | `framesReused / (framesPainted + framesReused)` | 同上 |
| `workFps` | 引擎**产生**帧的速率 | — |
| `presentFps` | 实际**呈现**速率 | 无 vsync 的 headless 下无意义 |
| `layersTotal` / `layersRastered` / `layersComposeOnly` / `layersStatic` | 层计数（R/C/S） | layer promotion 默认关闭，故常为 0 |
| `partialDamageEnabled` | 局部 damage 是否启用 | 默认关闭，见 `options.md` |
| `backend` | 后端标识 | — |
| `reflowCount` | 布局次数累计（**dll 纯增量计数器，不进公共头**） | — |
| `dmgFrame` / `autoDamage` | 自动 damage 的逐帧与聚合状态 | `KUI_DISABLE_AUTO_DAMAGE=1` 时退化 |
| `layoutViewport` | 布局视口 / surface / dpr / 根滚动偏移（3.56.0 新增） | — |
| `animationSources` | 是什么在持续请求帧（timer / transition / animation / lottie / view3d / raf） | 全 0 = 已进入按需渲染 |

`autoDamage` 子字段：`enabled`、`eligible`、`partialFrames`、`fullFrames`、`mirrorRebuilds`。
`partialFrames / (partialFrames + fullFrames)` 才是「局部重绘命中率」，`skipRate` 是
另一件事（整帧跳过）。两者不要混用。

## 4. 录制 mode：合法值与「静默提升」规则

合法 mode 只有四个，权威来源是 `Performance.getCapabilities` 的 `modes`：

```
summary  <  timeline  <  attribution  <  deep-trace
```

**mode 会被自动提升，这是设计行为不是 bug**（consumer 在 R2-G4 里踩到了这一点）：

- 你请求的 mode 决定**最低**采集档位；
- 但 `categories` 里任何一项的**视图要求**高于该档位时，引擎会把 mode 提升到能满足它的
  最低档，并在 `getRecording` 的 `effectiveConfig.mode` 里如实回报**提升后**的值；
- 依据写在 `getCapabilities` 的 `categorySemantics` 字段：
  `view-requirements-promote-mode-not-independent-hot-path-filters`。

所以：**永远以 `effectiveConfig.mode` 为准，不要假设它等于你请求的 mode。**
典型例子：请求 `timeline` 但带上 `nodes` 类目，实际会跑成 `attribution`。

传入词表外的值（如 `standard`）会得到 `-32602 Invalid params: unknown performance mode`。

## 5. 采集建议

- 机器无关、可做硬门禁的量：`reflowCount` 的前后 delta、`frames[].work.*` 的各项计数。
  这些是**工作量**，不随机器波动。
- 机器相关、只能做同机相对对比的量：`frameMs`、`workFps`、`presentFps`、各 phase 耗时。
  headless 无 vsync 时 `presentFps` 尤其不可用。
- 录制默认用 `summary`；`attribution` 会禁用卡片缓存，本身就会改变被测对象。

---

## 6. 这份文档的维护方式

字段清单目前是手写的，这是已知风险——`docs/devtools.md` 当年就是因为手抄工具数量而
出现「66 / 79 / 77 三个数」。已登记 `T-DEBT-DEVTOOLS-COUNTER-FIELD-CONTRACT`：目标是由
`Performance.getCapabilities` 同源生成字段级字典，与本文件合并。在那之前，
**以引擎实际返回为准**，发现本文件与实际不符请直接报。
