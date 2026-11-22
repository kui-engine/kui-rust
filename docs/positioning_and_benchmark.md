# KUI 官方定位与对标口径声明

> 本文件是 KUI 对外**唯一可引用**的定位声明。任何关于「这是缺陷还是设计边界」的
> 判定，以本文件为准。与本文件冲突的其它文档（包括历史 README、FAQ、教程和头文件
> 注释）一律作废，发现即按本文件修正。
>
> - 生效版本：**3.57.0**
> - 状态：生效
> - 修订流程：见 §7

---

## 1. 一句话结论

**KUI 的目标是与浏览器对等。唯一按设计不对齐的是脚本语言：KUI 用嵌入式 Lua，不实现
ECMAScript。**

除此之外的任何可观察差异——布局几何、绘制结果、DOM/CSSOM API 的返回值、事件时序、
CSS 属性支持——**都按缺陷处理**，区别只在优先级和排期，不在正当性。

这条口径在 3.57.0 生效，取代此前「KUI 面向原生应用 UI，不承诺完整 Web 平台兼容」
的旧表述。旧表述曾被用作大量差异的结项依据，那些结项需要按本文件重新定性（见
`docs/known_issues.md`）。

---

## 2. 标杆

| 项 | 值 |
|---|---|
| 标杆浏览器 | **Google Chrome（stable 频道）** |
| 当前对标基线版本 | **153.0.8010.48**（Windows x64） |
| 标杆平台 | Windows 10/11 x64，系统缩放 100%，viewport 由用例显式指定 |
| 次级参考 | Firefox / Safari 仅在「Chrome 疑似自身偏离规范」时引入，用于三方对照 |

**为什么是 Chrome 而不是规范字面**：规范在很多处留有实现自由度，而 consumer 拿 KUI
替换的是「用户预期像浏览器一样工作的界面」。当 KUI 更贴规范字面、Chrome 另有行为时，
**以 Chrome 为准**，除非本文件 §4 明确登记了该处刻意分叉。这一条直接推翻了此前
「KUI 更符合规范字面，故非缺陷」这类结项理由。

**版本漂移怎么办**：Chrome 每 4 周一个 stable。基线版本号由本文件维护，不随浏览器
自动升级；升级基线是一次显式动作，须重采 golden 并记录 provenance（见 §6）。

---

## 3. 逐层对标声明

| 层 | 是否对标 | 说明 |
|---|---|---|
| HTML 解析与 DOM 树构造 | ✅ 对等 | 含容错解析、隐式标签补全、表格 foster parenting |
| CSS 解析与级联 | ✅ 对等 | 含 specificity、`!important`、`@layer`、自定义属性与 `calc()` |
| 布局 | ✅ 对等 | block / inline / flex / grid / table / 绝对定位 / 浮动 |
| 绘制 | ✅ 对等 | 几何与颜色对等；亚像素抗锯齿与字形栅格化结果允许像素级差异（见 §4.3） |
| DOM API 语义 | ✅ 对等 | 返回值、异常、副作用与时序都对标 |
| CSSOM / CSSOM View 语义 | ✅ 对等 | `getBoundingClientRect` / `scrollWidth` / `getComputedStyle` 等返回值对标 |
| 事件模型 | ✅ 对等 | capture→target→bubble、pointer capture、默认动作、事件顺序 |
| **脚本语言** | ❌ **按设计不对齐** | 见 §4.1 |
| Web 平台 API 面（scripting 可触及的部分） | ✅ 对等（目标） | 见 §4.1 的说明：语言不同，**API 面与语义对等是设计目标** |
| 网络栈与远程资源 | ⚠️ 部分 | 见 §4.2 |
| 无障碍（AT 树） | ⚠️ 部分 | 见 §4.2 |
| 性能 | ❌ 不作对标声明 | 见 §5 |

---

## 4. 例外清单

**本清单是穷举的。** 不在本清单上的差异一律是缺陷。新增例外必须走 §7 的流程并落成
ADR，不能在代码注释、工作日志或 issue 回复里临时宣布。

### 4.1 按设计不对齐（唯一一项）

**脚本语言**。KUI 嵌入构建依赖提供的 Lua，不实现 ECMAScript，也**不提供 JS→Lua 迁移或兼容层**，
且无此计划。Lua 小版本不由本文固定；以实际引擎返回的 `_VERSION` 和该 SDK 的构建记录为准。

推论，逐条明确：

1. **依赖内联/外链 JS 的真实网页无法直接运行。** 「对标浏览器」不含「能打开任意网站」。
   把网页移植到 KUI 需要把脚本改写为 Lua。
2. **但 Lua 能触及的 API 面与语义，对等是设计目标。** `document.querySelector`、
   `element.getBoundingClientRect`、`addEventListener`、`MutationObserver`、
   `IntersectionObserver`、`ResizeObserver`、Canvas 2D、WebGL 等绑定，其**行为**应与
   浏览器一致；命名按 Lua 惯例（`obj:method()`）而非 JS 惯例，这属于语言差异，不属于
   语义差异。绑定覆盖面的缺口按缺陷处理。
3. **不对标的是语言本身**：Promise/async、Proxy、模块系统、原型链等 JS 语言特性没有
   对应物，也不承诺有。

### 4.2 已知能力缺口（是缺陷，不是例外；列在这里是为了让 consumer 不必重复上报）

这些**不是**设计边界，只是尚未实现。完整状态见 `docs/known_issues.md`：

- 多列（`column-*`）、CSS Shapes（`shape-outside` 等）、`grid-template: subgrid`：这三项**没有被编译开关禁用**——
  `KUI_DISABLE_MULTICOL` / `KUI_DISABLE_SHAPES` / `KUI_DISABLE_SUBGRID` 只是声明性的范围标记，源码里没有任何地方读取它们。
  多列与裸 `subgrid` 有可运行的实现，但没有完整规范一致性；`shape-outside` / `shape-margin` / `shape-image-threshold` 没有实现
  （`clip-path` 的 `circle()` / `ellipse()` / `inset()` / `polygon()` / `rect()` / `xywh()` 可用，`url()` / `path()` / `shape()` 不支持）。
  依据 `docs/adr/ADR-001-out-of-scope-features.md`；**ADR-001 的定性在 3.57.0 起
  由「永久范围外」改为「未实现，待排期」**，逐项范围见 `docs/css_feature_matrix.md`。
- Flex / Grid / Table 的分页 fragmentation。
- 远程字体（`@font-face { src: url() }` 下载）、`unicode-range` 的码点级应用、
  `font-display` 的可观察行为——三者同源于「当前只有同步的本地字体加载模型」。
- 3D transform（`rotateY` / `perspective` 等 CSS Transforms Level 2 能力）。
- 无障碍树的完整性（当前经 DevTools 可读，未接系统 AT）。

### 4.3 天然不同项（不构成缺陷，但也不是「随便差多少都行」）

只有以下三类允许与 Chrome 不一致，且每类都有明确边界：

| 项 | 允许的差异 | 不允许的差异 |
|---|---|---|
| **字形栅格化** | 抗锯齿算法、hinting、亚像素定位导致的**像素级**明暗差 | 字形**推进宽度**、行盒高度、基线位置的差异——这些是度量，属于布局，必须对等 |
| **原生窗口装饰** | 系统标题栏、窗口阴影与圆角的实现方式与平台外观 | 无（这部分不由 CSS 描述） |
| **原生控件外观** | 由宿主平台绘制的部分（如系统下拉列表弹出层的外观） | **UA 默认样式表的值**（margin/padding/border 等）必须与 Chrome UA 对齐 |
| **滚动条宽度与占位** | 厚度（KUI 固定 8px）、是否占布局 gutter 与 Chrome 不同；由此产生的可用宽度/断行差异 | 同一滚动条在布局、CSSOM、绘制、命中测试四处厚度不一致；嵌套容器错误累积 |

关于滚动条（2026-10-10 用户裁决，取代此前「滚动条分叉属于缺陷」的说法）：**滚动条的宽度及其占位**
**不要求与浏览器对齐**，属于天然不同项。KUI 在所有样式档位（含 BrowserCompat）保持紧凑 8px
（`src/layout/scrollbar_metrics.h`），与 Chrome 在 Windows 上 classic 滚动条的 gutter（约 13～15px）
之差不再视为缺陷，也不为此更新像素基线。仍然成立的只有**内部一致性**：同一个滚动条的厚度必须在
布局、CSSOM 几何、绘制和命中测试四处共用同一个数，且嵌套容器按各自的厚度占位、不得错误累积。
与 Chrome 对照时，凡由滚动条宽度引起的可用宽度、断行差异，一律记为观察，不登记为缺陷；
注意 headless Chrome 默认隐藏滚动条，不能作为 classic 滚动条的对照依据。

### 4.4 当前 `:empty` 空白策略

当前实现忽略 Comment 和纯空白 Text 子节点：空元素、仅注释、仅空白文本以及注释与
空白文本组合均匹配 `:empty`；非空白文本或元素子节点不匹配，`display:none` 的元素
子节点仍算内容。这是当前 KUI 的匹配行为，与 Chrome 将空白文本算作内容的结果不同。
消费者不能用 `:empty` 判断“完全没有 DOM 子节点”。MRG-57 修正的是注释误阻止匹配；
本轮按用户要求保留现有空白策略，没有完成与 Chrome 的空白语义对齐，也不据此宣称
全部 Selectors 行为一致。专项证据见 MRG-57。

### 4.5 当前 HTML `tagName` 返回值

KUI 的 HTML 元素 `tagName` 当前返回小写名称，例如 `p`、`div`，Lua 绑定原样返回；
这与浏览器 HTML DOM 常见的 `P`、`DIV` 不同。本轮保留已有公开返回值，未将此差异改为
大写接口。跨实现比较 HTML 标签时应显式规范化字符串，Lua 用法见
[Element 身份属性](lua.md#42-the-element-usertype)。这一建议限于 HTML
标签比较，不能由此推导 SVG、自定义元素或其他名称均可任意改写大小写。原页验证见
MRG-58。

### 4.6 布局资源护栏与不完整几何

布局派发的逻辑递归深度上限为 **256**（`LayoutContext::kMaxLayoutDepth`），这是布局
执行深度，不是 HTML 解析器的 DOM 深度上限，也不保证每种盒树都可完整布局 256 层
DOM。每趟还有 **50,000** 次布局工作量预算；计数超过预算时停止下探，重复布局也计入。
物理栈余量不足时会更早停止，增大宿主栈不能取消逻辑深度或工作量护栏。

触发任一护栏时，每趟首次停止在 stderr 输出 `[KUI-LAYOUT-GUARD] incomplete=true`，
并带 `reason=depth|box-budget|stack`、当前 depth/boxes 与 max_depth/max_boxes；该趟
后续停止不重复刷日志。DOM 节点仍可查询，但受影响子树的布局几何可能不完整或为零，
不能把读到的边界当成完整排版结果。消费者应降低嵌套/布局工作量并重新布局，不应通过
关闭诊断或提高阈值掩盖截断。原 601 个 div 页保留全部节点而叶子高度为零的验证及
诊断修复见 MRG-59。

---

## 5. 性能不在对标范围

`docs/architecture.md` 里「集显典型 UI 60+ FPS；静态 UI 144+ FPS」是**绝对工程目标**，
不是相对 Chrome 的对标目标。本文件不声明 KUI 与 Chrome 的任何性能关系。

consumer 侧能给出的性能结论，只有**同机器、同构建对比的相对回归**；绝对性能 PASS
需要 KUI 侧 clean release 的双目标机 campaign 与物理矩阵聚合门，这个条件 consumer 侧
不具备，因此 **consumer 不应给出绝对性能 PASS/FAIL 结论**。可稳定使用的是机器无关的
工作量指标（如每次交互的布局次数）。

---

## 6. 对标证据怎么产生

| 证据 | 位置 | 说明 |
|---|---|---|
| WPT 一致性 | `docs/wpt_conformance_snapshot.md` | 由发布门逐项比对的精确基线折算，随包交付 |
| CSS 特性矩阵 | `docs/css_feature_matrix.md` | 「KUI 自认为支持什么」的权威表，随包交付 |
| 浏览器对照用例 | `adv_examples/` + 其 `golden/geometry.json` | golden 用 §2 的标杆浏览器采集 |
| golden 采集环境 | 各 `golden/provenance.json` | 3.57.0 起随采集写入：浏览器版本、viewport、DPR、系统缩放、字体指纹、采集时间与提交 |

**3.57.0 之前采集的 golden 没有 provenance**，因此无法事后证明它们用的是哪个 Chrome
版本。这些旧 golden 在重采之前只能当作「参考」，不能当作「基线漂移的证据」。

---

## 7. 修订流程

1. 新增或修改 §4 的例外，必须先提 ADR（`docs/adr/`），经 KUI 维护者评审后才能改本文件。
2. 抬升 §2 的标杆浏览器版本，必须同时重采受影响的 golden 并更新其 provenance。
3. 本文件每次修改必须改顶部的「生效版本」，并在 `CHANGELOG.md` 留条目。
4. consumer 引用本文件时请连版本号一起引用，例如：
   「依据 `docs/positioning_and_benchmark.md`（3.57.0）§4.3，字形栅格化的像素级差异
   不构成缺陷，但推进宽度差异构成缺陷。」

---

## 8. 与其它文档的关系

| 文档 | 关系 |
|---|---|
| `docs/features.md` | 能力**清单**；定位口径以本文件为准 |
| `docs/faq.md`、`docs/tutorial.md` | 面向上手，不承担定位口径职责 |
| `docs/adr/ADR-001-out-of-scope-features.md` | 记录范围外特性（多列、CSS Shapes、subgrid 等）的历史决策与当前定性 |
| `docs/known_issues.md` | 已知缺陷与其状态的唯一汇总入口 |
| `include/kui/config.h` | 声明 `KUI_DISABLE_*` 范围标记（声明性，不改变行为；只在原生 C++ SDK 的头文件里） |
| `feedback/` | consumer 提交反馈的流程与模板 |
