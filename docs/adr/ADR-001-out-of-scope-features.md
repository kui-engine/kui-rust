# ADR-001: OUT_OF_SCOPE 特性取舍（GUI 子集边界）

> ## ⚠️ 已被 3.57.0 的定位声明修订（2026-09-22）
>
> **本 ADR 的前提已经变了。** 它成文时 KUI 的定位是「C++ GUI 框架，用 HTML/CSS 子集
> 描述界面」，因此把「网页/印刷场景高频、桌面 GUI 场景罕见」当作判定 OUT_OF_SCOPE 的
> 依据。自 3.57.0 起，KUI 的口径是**与浏览器对等，唯一按设计不对齐的是脚本语言**，见
> [`../positioning_and_benchmark.md`](../positioning_and_benchmark.md)。
>
> 因此：
>
> - 本 ADR 里被判为 `OUT_OF_SCOPE` / `DEFERRED` 的特性（多列、CSS Shapes、subgrid 等）
>   **重新定性为「未实现，待排期」**，不再是「设计边界」。
> - 对应的 `KUI_DISABLE_*` 开关只是声明性的范围标记，源码里没有任何地方读取它们，所以 3.57.0 **行为不变**，
>   开或关都不会禁用或启用代码路径（多列与裸 `subgrid` 有可运行的实现）。变的是定性，不是行为。
> - 引用本文件作为「这不是缺陷而是设计」的依据**不再成立**。逐条状态请查
>   [`../known_issues.md`](../known_issues.md)。
>
> 正文原样保留，作为当初决策背景与取舍理由的历史记录。

## 元信息

| 字段 | 值 |
|---|---|
| 编号 | ADR-001 |
| 作者 | KUI Core Team |
| 日期 | 2026-04-17 |
| 状态 | Accepted |
| 关联 Roadmap 项 | R-10（GUI 子集决策落地）；D-05 后由 R-23B/R-23C/R-23D、R-24A、R-25C 修订 |

---

## 状态

**Accepted**

---

## 背景

KUI 定位为 **C++ GUI 框架**，使用 HTML + CSS 子集作为声明式 UI 描述语言。项目当前 WPT 通过率 67.7%，目标提升到 95%。但在审查中发现：

1. 部分 CSS 特性（如 CJK 竖排、多列布局、`clip-path: shape()`、`subgrid`、`@page`）**仅在网页/印刷场景**中高频使用，**在桌面/移动 GUI 场景中罕见**
2. 实现这些特性的工程成本高（每项 10–20 人日），但对 KUI 核心场景 ROI 极低
3. 不明确范围会导致 Roadmap 无限膨胀，永远无法达到"完成"状态

需要一次性明文决定哪些特性 **永久排除** 或 **推迟到 3.0 之后**，并通过 CI 与文档约束。

### 证据

- WPT 中 `css-writing-modes`、`css-multicol`、`css-shapes`、`css-pagination`、`css-grid/subgrid` 套件总计约 800 个用例，占总量 14%
- Blink / Gecko 实现这些特性耗时多年（公开历史代码可查）
- KUI 的典型应用场景：桌面工具、嵌入式 HMI、游戏内 UI、移动 App，均不依赖以上特性

---

## 决策

我们决定对以下 CSS 特性做 **OUT_OF_SCOPE** 或 **延期** 处理，并通过 `docs/css_feature_matrix.md` 明文固化，通过 CMake feature flag 控制编译，通过 `scripts/wpt_diff_report.py --scope` 从总体通过率中剔除。

### D-01 · Writing Mode（CJK 竖排 / RTL 完整流）

**决定**：**OUT_OF_SCOPE / 1.0**

- `writing-mode: vertical-rl / vertical-lr / sideways-rl / sideways-lr` 不实现
- `direction: rtl` 仅对文本双向算法支持，不支持完整的 RTL 布局流
- `text-orientation` 不支持

**理由**：
- 桌面 / 移动 GUI 场景中 RTL 由文本层自动处理（HarfBuzz + ICU BiDi），完整的 RTL 布局流成本极高
- CJK 竖排在现代 GUI 中几乎无使用场景
- 若未来需要（如电子书应用），可独立成 3.0 新模块

### D-02 · Multi-column Layout（多列布局）

**决定**：**OUT_OF_SCOPE / 1.0**

- `column-count`、`column-width`、`column-gap`、`column-rule`、`column-span`、`column-fill`、`break-*` 系列全部不实现

**理由**：
- 典型网页排版特性，GUI 中罕见
- 实现复杂度高（跨列破坏、断点算法、列间协调）

### D-03 · CSS Shapes

**决定**：**OUT_OF_SCOPE / 1.0**

- `shape-outside`、`shape-margin`、`clip-path: polygon() / path() / shape()`、`border-shape: shape()` 不完整实现
- `clip-path: inset() / circle() / ellipse() / rect()` **保留**（简单且常用）

**理由**：
- `polygon()` / `path()` 渲染与 hit-testing 复杂度高
- GUI 场景用 `clip-path: inset()` 与 `border-radius` 已足够

### D-04 · Subgrid

**决定**：**DEFERRED_TO_3.0**

- `grid-template: subgrid` 值不实现

**理由**：
- 依赖 CSS Grid Level 2 完整规格，实现成本 15+ 人日
- 基础 Grid（Level 1）已足够覆盖 GUI 需求
- 若 Grid Level 1 稳定后有明确用户需求，再加入 3.0

### D-05 · Print / Pagination

**决定**：**PARTIAL / 已修订（2026-09-10）**

2026-04-17 的“打印/分页永久排除”决定已被后续实现事实取代，不能再作为当前能力声明。当前已纳入并实现的安全子集包括：

- `Document::printToPDF()` 与 SkPDF 多页导出；
- 导出期间启用 `@media print`；
- `@page` 的页面几何及现有页规则/边距盒子集；
- Block + IFC 的基础 `Fragmenter`；
- `break-before` / `break-after` / `break-inside` 及 `page-break-*` legacy alias 的基础分页消费。

当前仍不宣称支持的边界是：

- 浏览器 WPT print reftest 的逐页捕获与 KUI PDF 逐页栅格对照 harness；普通 screen 四图相等不能证明打印语义正确；
- paged multicol，以及 Flex/Grid/Table 的专用 fragmentation；其中包括 table-row 跨页、重复 `<thead>` 等语义；
- `orphans` / `widows`、复杂嵌套 break 冲突等高级分页规则；
- 系统打印对话框；当前产品入口是 `printToPDF()` 后交给外部打印器。

因此 D-05 不再允许按“包含 `@page`、`_print` 或 pagination”整套排除。WPT 必须按缺失的具体分页语义分类；待逐页 print harness 建立后，才能用 print reftest 判断实现与收编状态。

### D-06 · Container Queries 进阶特性

**决定**：**DEFERRED_TO_3.0**

- `@container (width > 500px)` 基础查询 **保留规划**（M5 里程碑）
- `container-type: size` 的大小查询 **保留规划**
- `@container style(--var: value)` 样式查询 **延期**

### D-07 · `@scope` 规则

**决定**：**DEFERRED_TO_3.0**

- `@scope (.card) { ... }` 不在 2.x 版本实现

### D-08 · 复杂滤镜

**决定**：**OUT_OF_SCOPE / 1.0**

- `filter: url(#svg-filter)` SVG 引用不支持
- `filter: blur() / brightness() / contrast() / ...` 的 CSS 内置滤镜 **保留**

**理由**：SVG filter 依赖完整 SVG 渲染管线，成本超过 30 人日。

---

## 后果

### 正面

- **Roadmap 范围可控**：总工作量从"∞"收敛到 117–151 人日
- **WPT 目标可达**：剔除 OUT_OF_SCOPE 套件后，IN_SCOPE 范围的 95% 通过率可实现
- **文档与代码一致性**：CMake flag 确保未来开发者不会误打开相关代码
- **降低维护负担**：避免长期维护使用率极低的特性

### 负面

- **用户误解风险**：用户可能以为"CSS 都支持"，需要明文在 README 与 `docs/css_feature_matrix.md` 说明
- **未来需求变化成本**：若某特性未来成为刚需，需要额外 ADR 重启（非零成本）
- **WPT 绝对通过率下降**：总体通过率（包含 OUT_OF_SCOPE）会低于去除后的数值，需要报告分层

### 中立

- 对 KUI 核心用户（桌面 / 移动 GUI 应用开发者）**无实际影响**
- 对竞品差异化（vs Electron / Qt）的定位更清晰

---

## 备选方案

### 方案 A：全量实现（对齐浏览器）

**描述**：按 CSS Working Draft 全量实现所有特性。

**拒绝原因**：
- 工作量不可控（估算 500+ 人日）
- 与"GUI 框架"定位不符
- 长期维护成本高（WG 规范持续演进）

### 方案 B：仅隐藏在 feature flag 后（不声明范围）

**描述**：代码保留但默认不编译，不做明文声明。

**拒绝原因**：
- 用户无法知悉范围
- Roadmap 无法收敛
- CI 无法做总体通过率有意义的度量

### 方案 C：按需实现（用户请求驱动）

**描述**：不做 OUT_OF_SCOPE 决定，等用户提需求再实现。

**拒绝原因**：
- 无明文取舍会让 Roadmap 永远"看起来不完整"
- 影响 2.x 稳定发布
- 对核心场景不公平（核心特性因分配给长尾特性的资源而延期）

---

## 验证与跟踪

| 指标 | 期望值 | 实测 | 时间点 |
|---|---|---|---|
| IN_SCOPE WPT 通过率 | ≥ 95% | TBD | Roadmap 完成时 |
| `css_feature_matrix.md` 标注完整性 | 100% | TBD | R-10 完成时 |
| CMake feature flag 生效 | 3 个 flag 可用 | TBD | R-10 完成时 |
| 用户反馈 OUT_OF_SCOPE 投诉 | ≤ 2 条 / 季度 | TBD | 3.0 发布后 |

---

## 参考

- Roadmap 章节：`docs/KUI_彻底治理_Roadmap.md` §二.2 D-01 ~ D-08
- 特性矩阵：`docs/css_feature_matrix.md`（R-10 交付）
- CMake flags：`CMakeLists.txt` 中 `KUI_DISABLE_MULTICOL` / `KUI_DISABLE_SHAPES` / `KUI_DISABLE_SUBGRID`（R-10 交付）
- WPT 脚本：`scripts/wpt_diff_report.py --scope`（R-10 扩展）
