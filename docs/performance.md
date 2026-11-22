# 性能相关的特性与开关

本文列出与应用开发直接相关、默认开启的几项性能特性：它们解决什么问题、应用要做什么、代价是什么、怎么关。
每一项都有对应的环境变量开关（kill-switch），在进程里第一次用到时读取，用于排查问题或对比；正常使用不需要设置。
各版本的完整变化见 [`../CHANGELOG.md`](../CHANGELOG.md)。

| 特性 | 平台 | 起始版本 | 应用要做的事 | 关闭 |
|---|---|---|---|---|
| [`kui-preload` 标记](#1-kui-preload提前备好还没打开的界面) | Windows 窗口，GPU 绘制 | 3.60.0 | 给还没打开的弹窗加标记 | `KUI_DISABLE_PRELOAD_WARMUP=1` |
| [GPU 程序磁盘缓存与预编译](#2-gpu-程序磁盘缓存与首帧后预编译) | Windows / Linux，GPU 绘制 | 3.60.0 | 无 | `KUI_DISABLE_SHADER_CACHE=1` |
| [`<template>` 内容惰性化](#3-template-的内容是惰性的行为变化) | 全平台 | 3.60.0 | 依赖旧行为的页面要改 | 无（按 HTML 规范的行为） |
| [窗口先画好再现身](#4-窗口先画好再现身) | Windows | 3.60.0 | 无 | `KUI_DISABLE_SHOW_WHEN_READY=1` |
| [CSS 变量沿继承共享](#5-css-变量沿继承共享) | 全平台 | 3.61.0 | 直接读 `StyleExtras` 变量字段的 C++ 代码要改 | `KUI_DISABLE_SHARED_CUSTOM_PROPERTIES=1` |
| [增量排版的几处优化](#6-增量排版的几处优化3611--3612--3614) | 全平台 | 3.61.1 / 3.61.2 / 3.61.4 | 无 | 见第 6 节 |
| [动画与悬停的几处优化](#7-动画与悬停的几处优化3614) | 全平台（悬停一项仅 Windows） | 3.61.4 | 无 | 见第 7 节 |

## 1. `kui-preload`：提前备好还没打开的界面

**问题。** 一块界面（弹窗、抽屉、设置页）第一次画出来时，显卡要现场编译它用到的着色器程序。所以这块界面在这台机器上第一次打开时
会卡一两帧：显卡驱动缓存也是冷的时候，实测第一次打开订单详情弹窗，最重的两帧是 45 + 37 ms。

**用法。** 把还没打开的界面放进带 `kui-preload` 属性的 `<template>`：

```html
<template kui-preload>
  <dialog class="business-modal">
    <header class="modal-head">订单详情</header>
    <table class="detail-table"><tr><td>占位</td></tr></table>
    <footer><button class="primary">确定</button></footer>
  </dialog>
</template>
```

也可以把属性加在页面里暂时隐藏的元素上，例如 `<div class="drawer" hidden kui-preload>…</div>`。

**KUI 做了什么。** 页面加载后第一帧上屏、窗口现身之后，KUI 在同一个 UI 线程上另建一个看不见的文档：用页面已经解析好的全部样式表，
把带标记的内容画两遍（一遍是正常状态，一遍是动画中途的半透明、缩放状态），画到离屏的 GPU 画布上，顺带把这些界面要用的程序编好。
每个文档只做一次，用完立即释放，不碰页面真实的 DOM、布局和绘制。之后用户真正打开这块界面时，程序已经编好。实测上面那个弹窗
第一次打开的最重两帧从 45 + 37 ms 降到 13 + 3 ms。

**写标记时注意：**

- 离屏文档把带标记的内容放在 `<body>` 的第一层。依赖祖先的选择器（例如 `.sidebar .panel`）匹配不到，需要的外层元素请一起放进模板。
  `<html>` 和 `<body>` 上的属性（class 等）会复制过去。
- 内容只要结构与 class 和真实界面一致即可，数据可以是占位文字。
- 里面的 `<dialog>` 一律按模态方式打开（遮罩层也会编）。动画与过渡在离屏文档里关闭。
- 离屏文档没有 Lua 脚本引擎，也没有自定义元素注册。
- 只在 Windows 窗口上生效；使用 CPU 绘制（没有 GPU）时不做。其他平台上这个属性没有作用，也没有副作用。
- `<template>` 的内容本来就不显示、查不到（见第 3 节）；标在隐藏元素上时，元素在页面里照旧隐藏。

**代价。** 窗口现身之后 UI 线程会停顿一次：缓存热时约 48 ms，驱动冷时约 140 ms（主要是原本第一次打开时要付的编译时间，挪到了这里）。
启动后进程私有内存多约 50 MB，这是把打开这些界面时本来要占的内存提前占了——界面都用过之后两种情况只差 0～3 MB；但被标记的界面
如果始终没被打开，这部分就白占了。**所以只标用户多半会打开的界面。**

**开关。** `KUI_DISABLE_PRELOAD_WARMUP=1` 关闭；`KUI_PRELOAD_WARMUP_DIAG=1` 往 stderr 打印每次预热的片段数、遍数和分段耗时。

## 2. GPU 程序磁盘缓存与首帧后预编译

**做了什么。** GPU 绘制（OpenGL 后端，Windows / Linux）时，KUI 把编过的程序存到磁盘。下次启动时，第一帧上屏之后再把以前用过的程序
提前编好，于是第一次打开弹窗之类的操作不再现场编译。这和 Chrome 的 GPU 程序缓存是同一个思路。应用不需要做任何事。

**与 `kui-preload` 的关系。** 磁盘缓存只覆盖「这个程序在这台机器上以前用过的」界面；某块界面在这台机器上第一次出现时，要靠 `kui-preload`。

**缓存位置。** Windows 默认是 `%LOCALAPPDATA%\KUI\ShaderCache`，可以用 `KUI_SHADER_CACHE_DIR` 改。子目录按显卡厂商与型号、GLSL 版本、
KUI 版本和程序名（exe 文件名）分开，只保留最近 8 个。文件带校验并原子写入，读出来对不上的条目当作没有并删除。

**代价。** 磁盘每个程序约 4～5 KB（一个中等规模页面约 30 个程序、137 KB；每个子目录上限 64 MB）。启动后的预编译在第一帧之后执行，
驱动热时约 10～18 ms，驱动冷时约 90 ms。常驻内存只有目录与统计。

**开关。** `KUI_DISABLE_SHADER_CACHE=1` 缓存与预编译都关；`KUI_DISABLE_SHADER_PRECOMPILE=1` 只关预编译；
`KUI_SHADER_CACHE_DIAG=1` 打印预编译统计。

## 3. `<template>` 的内容是惰性的（行为变化）

3.60.0 起 `<template>` 按 HTML 规范处理：模板里的内容不是模板元素的子节点，所以

- 不显示；
- `getElementById` / `querySelector` 查不到；
- 里面的 `<style>` / `<link>` 不生效，脚本不执行；
- 读写模板的 `innerHTML` 得到 / 替换的是模板内容；深克隆（`cloneNode(true)`）连内容一起复制。

此前 KUI 把模板内容当普通子树：会显示出来，里面的 id 还会和页面上真正的元素抢位置。

**要改的页面。** 如果页面依赖模板内容可见或能被查到，改用普通的隐藏元素（例如 `<div hidden>`）。要把模板实例化，读模板的
`innerHTML`，赋给目标容器的 `innerHTML`（或插入它的深克隆）。

**例外。** 声明式 Shadow DOM 的 `<template shadowrootmode="open">` / `"closed"`：KUI 不实现 Shadow DOM，这种模板的内容仍然原地显示
（与浏览器里内容成为 shadow root 后的显示效果一致）。

## 4. 窗口先画好再现身

**做了什么（Windows）。** 窗口从隐藏变为显示时，KUI 先让窗口在屏幕上保持不可见，当场画好第一帧，等它真正上屏后再让窗口现身，
不再先闪一下白色。以 `visible = true` 创建、还没有加载页面的窗口，会等页面加载后的第一帧画好才出现；最多等 1 秒，超时直接显示。

**代价。** 每次从隐藏变为显示多一次整窗重画和一次合成等待（不超过一个屏幕刷新周期）；无常驻内存。

**开关。** `KUI_DISABLE_SHOW_WHEN_READY=1` 回到旧行为（立即显示）。

## 5. CSS 变量沿继承共享

**做了什么。** 以前每个元素继承 CSS 变量（自定义属性）时，都会把父元素的整张变量表复制一份；在 `:root` 上定义几十个主题变量的页面，
这份复制约占样式计算时间的三成。现在继承只共享同一份变量表，元素自己声明或改写变量时才复制。实测（根上 29 个变量）弹窗打开那一帧
样式计算 5.2 → 4.1 ms，内存还少 2～5 MB。写 CSS 的方式不用变。

**C++ 代码的影响。** 公共头 `kui/css/style_extras.h` 里的 `StyleExtras` 不再有 `customProperties` / `customPropertyTokens` 两个成员，
改为共享的 `customVars`。读变量请用 `ComputedStyle::customProperties()` / `customPropertyTokens()`（签名与含义不变）。使用方需要重新编译；
C ABI 不受影响。

**开关。** `KUI_DISABLE_SHARED_CUSTOM_PROPERTIES=1` 回到逐元素复制。

## 6. 增量排版的几处优化（3.61.1 / 3.61.2 / 3.61.4）

这几项都是引擎内部的优化，应用不需要做任何事，也不改变排版结果。列在这里是为了排查问题时可以逐项关掉对比：如果怀疑某个排版问题与它们有关，
设对应的环境变量重启程序，看问题是否消失。

| 优化 | 效果（Forge 对话 / 日志查看器复刻页） | 关闭 |
|---|---|---|
| flex 容器里追加 / 删除子项走局部重建（元素子项一律按 flex 项计） | 30 轮历史的对话追加一条消息 498 → 15.7 ms | `KUI_DISABLE_PARTIAL_REBUILD_FLEX_ITEMS=1` |
| 刚插入、还没有盒的节点里继续增删，不再整树重建 | 渲染 30 轮历史 7858 → 1278 ms | `KUI_DISABLE_PARTIAL_REBUILD_BOXLESS_PARENTS=1` |
| 行内元素内容增删、带匿名盒的层级变化上提到最近的块级祖先局部重建 | 流式输出每批 p90 548 → 29 ms | `KUI_DISABLE_PARTIAL_REBUILD_ESCALATE=1` |
| 切换出现在 `::before/::after` 选择器里的 class 时只局部重建这一棵 | 日志行切选中态 41 → 36 ms | `KUI_DISABLE_GENERATED_CONTENT_LOCAL_REBUILD=1` |
| 不依赖块向约束的盒按归一后的键命中布局缓存 | 日志追加 20 行 55.5 → 49.4 ms | `KUI_DISABLE_BLOCK_SIZE_FREE_CACHE_KEY=1` |
| 没有元素换样式的帧跳过增量排版里的样式整树遍历 | 30 轮历史流式每批 24.6 → 20.6 ms | `KUI_DISABLE_SELECTIVE_STYLE_WALK_SKIP=1` |
| （3.61.2）本趟已经排过的盒，同一趟里再以相同约束派发时复用结果 | 日志整表重建 500 行 399 → 269 ms | `KUI_DISABLE_SAME_PASS_LAYOUT_REUSE=1` |
| （3.61.2）页面里没有「块进行内」拆分片段时，块排完不再整棵子树找空片段 | 30 轮历史只改尾段文字 13.75 → 11.71 ms | `KUI_DISABLE_IB_SPLIT_EDGE_WALK_SKIP=1` |
| （3.61.4）没有元素换计算样式指针的帧，跳过整棵 DOM 的样式所有权核对 | 30 轮历史只改尾段文字 12.4 → 10.6 ms | `KUI_DISABLE_STYLE_OWNERSHIP_WALK_SKIP=1` |

3.61.2 的同趟复用当时有一笔代价：大批新盒都只排一遍、没有可复用的帧（对话里一次追加一整段回复）约慢 1.5%，是记录「本趟排过哪些盒」的开销。3.61.4 起这份记录改记在盒子自己身上，这笔开销已测不出来，峰值内存也略省。

同一版还修了一个正确性问题：`display:contents` 元素上设置的 `color` / `font-size` 在增量排版之后会丢失、退回外层元素的值，现在已修复
（排查时可设 `KUI_DISABLE_DISPLAY_CONTENTS_TEXT_REBIND=1` 回到旧行为）。

## 7. 动画与悬停的几处优化（3.61.4）

与第 6 节一样，这几项都不需要应用做什么，也不改变画面结果；列出开关是为了排查时逐项关掉对比。

| 优化 | 效果（120 张卡片交错过渡等复刻页） | 关闭 |
|---|---|---|
| GPU 局部重放里，透明度层改用单个矩形钉层，不再每层先画一遍模板蒙版 | 首播首帧 20.6 → 17.1 ms，每帧合成 6.7 → 4.9 ms，内存不变 | `KUI_DISABLE_GPU_RECT_LAYER_PIN=1` |
| 三次贝塞尔缓动的查找表按曲线共享 | 点击帧样式计算 4.7 → 3.1 ms | `KUI_DISABLE_EASING_LUT_SHARE=1` |
| （Windows）动画由 vsync 驱动时，悬停引起的重画并入下一个刷新周期的帧 | 动画中每次移动都改变悬停样式时，额外的同步帧 20 → 0 | `KUI_DISABLE_HOVER_PAINT_VSYNC_COALESCE=1` |
| `outline-width` / `outline-offset` 的过渡按纯绘制处理 | `transition: all` 的元素被悬停加上描边，动画期间进布局的帧 16 → 0 | `KUI_DISABLE_OUTLINE_PAINT_ONLY=1` |

同一版的行为修正：独立 `translate` / `rotate` / `scale` 与 `filter` 的 `transition` 以前一帧跳到终点，现在连续播放（排查时可设
`KUI_DISABLE_INDIVIDUAL_TRANSFORM_FILTER_TRANSITIONS=1` 回到旧行为）。

**写页面时的建议**（这些写法在任何引擎里都更省）：长列表只追加新行、删掉最早的行，不要每次清空重建；一批更新里只读一次 `scrollHeight` 等
几何属性（每读一次都会让引擎当场排版）；先把整棵子树在文档外拼好再一次插入。

## 8. 文字整形复用与排版收尾（3.61.5）

与第 6、7 节一样，这几项都不需要应用做什么，也不改变排版结果；列出开关是为了排查时逐项关掉对比。

| 优化 | 效果（复刻页实测） | 内存 | 关闭 |
|---|---|---|---|
| 多 run 段落结果记忆表 128 → 1024 条；盒子已经不在、闲置超过 8 个工作趟的条目放掉 | 日志查看器整表重建 500 行，每次刷新 178.9 → 135.0 ms | 稳态约 +2.5 MiB | `KUI_DISABLE_MULTIRUN_MEMO_WORKING_SET=1` |
| 单段文字的 max-content 探测结果直接回答「按列宽排」的请求（一行放得下时） | 540 格表格首次打开首帧 142 → 108 ms | 无差异 | `KUI_DISABLE_LAYOUT_PARA_WIDTH_FREE_EXT=unconstrained` |
| 排版收尾的「同步滚动位置」只走可能滚动的元素 | 4 万盒页面流式改文字 11.2 → 9.2 ms | 无差异 | `KUI_DISABLE_SCROLL_SYNC_WALK_SKIP=1` |
| 排版收尾的「应用定位偏移」只走 relative / sticky / fixed 盒 | 同上再省约 0.8 ms | 无差异 | `KUI_DISABLE_POSITION_OFFSET_WALK_SKIP=1` |
| 排版收尾的「清排版脏位」只清登记的节点 | 同上再省约 1.2 ms（三项合计约 11.2 → 6.6 ms） | 无差异 | `KUI_DISABLE_DIRTY_CLEAR_WALK_SKIP=1` |

后三项都只在「盒树、DOM、样式自上一趟以来都没变」的帧生效（例如流式更新同一段文字）；插入 / 删除节点或改样式的帧照旧整树处理。

同一版的行为修正：flex / grid 里会收缩的 `overflow-y:auto` 滚动容器，首次排版或内容恰好跨过溢出阈值时当帧补排一趟（照 Chrome），结果不再依赖排版
历史（`KUI_DISABLE_TRIAL_SCROLLBAR_CONVERGENCE=1` 回到旧行为）。
