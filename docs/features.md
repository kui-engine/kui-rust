# KUI 能力边界

**KUI 的目标是与浏览器对等，唯一按设计不对齐的是脚本语言（Lua 而非 JavaScript）。**
完整口径见 [`positioning_and_benchmark.md`](./positioning_and_benchmark.md) —— 那是判定
「这是缺陷还是设计边界」的唯一可引用依据，本文件只是能力清单。

> 3.57.0 起口径变更：此前这里写的是「KUI 面向原生应用 UI，不承诺完整 Web 平台兼容」，
> 该表述已作废。按新口径，下面「当前缺口」一节里的项目**是待办的缺陷，不是设计边界**。

能力判断按以下来源，四者随包交付：

1. `include/kui/` 公共 API；
2. [`css_feature_matrix.md`](./css_feature_matrix.md)；
3. [`wpt_conformance_snapshot.md`](./wpt_conformance_snapshot.md)（由发布门逐项比对的
   精确基线折算，含每个套件的用例数与通过数）；
4. 目标平台的实际发布包和验证结果。

已知缺口与其排期见 [`known_issues.md`](./known_issues.md)。

## 当前主要能力

- HTML 解析与原生 DOM 操作；
- 常用 CSS 级联、选择器、变量、视觉属性；
- Block、Inline、Flex、Grid、Table 布局；
- Skia CPU/GPU 绘制、文本、图片与 Canvas；
- 事件、过渡、关键帧动画；
- 原生窗口、多窗口和若干系统集成；
- Lua、网络、DevTools、扩展与媒体接口；
- 默认开启的性能特性（`kui-preload` 预热、GPU 程序磁盘缓存、窗口先画好再现身等），用法、代价与开关见
  [`performance.md`](./performance.md)。

## 当前缺口（是待办，不是边界）

以下都**不是**设计边界，只是尚未实现。逐条状态与排期见
[`known_issues.md`](./known_issues.md)，历史决策背景见
[`adr/ADR-001-out-of-scope-features.md`](./adr/ADR-001-out-of-scope-features.md)：

- 多列（`column-*`）、CSS Shapes、subgrid 三项**没有被编译开关禁用**：`KUI_DISABLE_MULTICOL` / `KUI_DISABLE_SHAPES` /
  `KUI_DISABLE_SUBGRID` 只是声明性的范围标记，源码里没有任何地方读取它们，开或关都不改变行为。现状分别是：
  多列有可运行的实现（`column-count` / `column-width` / `column-gap` / `column-rule`、跨栏元素 `column-span`、分栏绘制），
  复杂嵌套与 fragmentation 的组合没有做到完整规范一致；subgrid 只支持裸 `subgrid`，把父轨道与 gap 差值传给子网格，
  `subgrid` 之后附命名线、子项向父级的 intrinsic 轨道贡献没有覆盖；CSS Shapes 的 `shape-outside` / `shape-margin` /
  `shape-image-threshold` 没有实现，`clip-path` 的 `circle()` / `ellipse()` / `inset()` / `polygon()` / `rect()` / `xywh()`
  可用，`url()`（SVG clipPath）、`path()`、`shape()` 不支持。
  这三项不在已承诺的范围内，**3.57.0 起定性由「范围外」改为「未实现，待排期」**；上面可运行的部分没有一致性承诺，
  逐项范围见 [`css_feature_matrix.md`](./css_feature_matrix.md)。
- Flex/Grid/Table 的分页 fragmentation 尚未实现（基础设施部分存在，当前只接通 block 路径供 PDF 用）；
- 远程字体（`@font-face` 的 `url()` 下载）、`unicode-range` 的码点级应用、`font-display`
  的可观察行为，三者同源于「当前只有同步的本地字体加载模型」；
- CSS 3D transform（`rotateY` / `perspective` 等）；
- 各平台的托盘、弹窗、输入控件和可选扩展不完全对称；
- 3D 使用独立 `kui_ext_3d`（CMake 组件名 `Ext3D`），旧仓内 3D 组件已停建；
- Go/Rust SDK 3.1.0 已达到 Windows x64 Full Profile GA，并各自交付 8 个 quickstart、
  225 个 canonical examples（211 个与 C++ 原版一一对应 + 14 个 API 巡览）和 233 个标准预编译程序（在预编译示例包里）；Linux x64 / macOS arm64 自 3.61.8 起有同口径的
  Release 包；`HttpClient` 在 Linux 上用运行期加载的系统 libcurl、在 macOS 上用 NSURLSession。

具体属性或标签是否支持，应检索当前矩阵和测试，不依赖历史版本的固定清单。
