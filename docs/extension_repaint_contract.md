# 扩展元素重绘约定（Extension Repaint Contract，扩展 API v1.11）

> **读者**：编写 KUI DLL 扩展（`kuiext_*` 协议）或原生 C++ 自定义元素（`CustomElementDefinition`）的开发者。
>
> **一句话**：扩展声明「画面一变就请求重绘」之后，宿主不再把它当成「来源未知」的重绘源——页面上别处的 hover、焦点、打字照样只重画变了的那一小块，
> 扩展自己的 `invalidateRect` 也成为精确脏区；代价是扩展必须严格遵守下面五条，并接受宿主把它的绘制裁在边框盒内。
>
> 规范性文件。设计背景见 KUI 仓库 `docs/design_扩展元素必请求重绘约定.md`；头文件说明见 `include/kui/extension/kui_extension_api.h` 的 v1.11 段。

## 1. 为什么需要它

KUI 的「小变化帧」只重画自上一帧以来变了的元素，硬线是**来源未知 ⇒ 整页重画**。带 `onPaint` / `onTick` / `onNeedsTick` 回调的扩展元素，
宿主无法证明「它的画面只在它请求重绘时才变」：回调可能每帧读外部可变状态（终端输出缓冲、后台线程结果），也可能画出自己的盒。
所以只要页面上有一个这样的元素，每一帧都整页重录绘制列表——hover 一个按钮、在别处输入框里打一个字都不例外。

## 2. 怎么声明

### 2.1 DLL 扩展

导出可选函数 `kuiext_paintContract`，返回 `KUI_EXT_PAINT_CONTRACT_*` 位（目前只有一位）：

```c
#include <kui/extension/kui_extension_api.h>

KUI_EXT_API uint32_t kuiext_paintContract(void) {
    return KUI_EXT_PAINT_CONTRACT_REPAINT_ON_CHANGE;
}
```

- 作用于本扩展注册的**全部**元素类型。
- 不导出、或返回 0：没有约定，行为与 v1.10 及以前完全相同。
- 兼容性：`KuiExtHostAPI::apiVersion` 不变（11）。比 v1.11 旧的宿主不认这个导出，等同没有声明；新宿主加载旧扩展也等同没有声明。
  `KuiExtRegistration` 与 `KuiExtCompatInfo` 都没有改动。

### 2.2 原生 C++ 自定义元素

```cpp
kui::CustomElementDefinition def;
def.tagName = "my-gauge";
def.paintCallback = ...;
def.repaintContract = true;   // 声明约定
doc->customElements()->define("my-gauge", std::move(def));
```

原生元素没有扩展宿主函数表，自己 `markNeedsPaint()` + `Document::scheduleRepaint()` 的请求**仍按来源未知处理**（那一帧整页，安全）；
声明约定的收益是「它在场不再拖累页面上别处的小变化」。`CustomElementDefinition` 末尾新增了 `repaintContract` 成员，C++ consumer 需要重新编译。

## 3. 约定内容（声明者必须全部满足）

1. **画面一变就请求重绘**：任何会改变 `onPaint` 输出的状态变化——输入回调（`onKeyEvent` / `onMouseEvent` / `onWheelEvent` / `onImeEvent`）、`onTick`、
   Lua 方法、`onMessage`、`onAttributeChanged`、后台线程结果落地……——都要在**宿主 UI 线程**上、**把控制权交还宿主之前**调用：
   - `scheduleRepaint(el)`：重画整个元素；或
   - `invalidateRect(el, x, y, w, h)`：只重画一块（content-box 局部坐标），矩形必须盖住**旧像素与新像素的并集**
     （光标从第 3 行移到第 7 行：两行都要；选区缩小：缩掉的那部分也要；水平滚动：整个可视宽度）。
2. **`onPaint` 只读元素自己的状态与属性**。读外部状态（全局缓冲、共享模型）的前提是：该状态的每一次变化都按第 1 条请求重绘。
   例如后台线程写入的数据，要在 UI 线程上取走（`onTick` 里轮询队列）并请求重绘，不能让 `onPaint` 直接读后台线程正在改的数据。
3. **只画在自己的边框盒内**。宿主对声明了约定的元素在 `onPaint` 外套一层裁剪（边框盒，即 content 原点向外加上 padding 与 border），
   画出去的部分不显示。需要伸出元素的内容（下拉列表、悬浮提示），请做成独立元素或原生弹窗。
4. **`onPaint` 不改 DOM / 样式 / 属性**（不调 `setAttribute` / `removeAttribute` 等）。改了的话，下一帧会被判为来源未知整页重画一次，但这一帧的画面可能不完整。
5. **焦点变化由扩展自己处理**：焦点进出若改变画面（光标、当前行高亮、焦点边框），扩展要能察觉（例如在 `onTick` 里比较上次绘制时的 `hasFocus`）
   并按第 1 条请求重绘；失焦后若 `onNeedsTick` 返回 0，要保证失焦那一刻的重绘已经发出。

## 4. 宿主做什么

| 情形 | 未声明（今天的行为） | 声明了约定 |
|---|---|---|
| 页面上有该元素时，别处的 hover / 焦点 / 打字 | 整页重录 | 照常只重画变了的部分 |
| `scheduleRepaint(el)` | 整页重录（自动 damage 再按差异上屏） | 只重画该元素（墨迹闭包） |
| `invalidateRect(el, …)` | 作为显式脏区补充，Painter 仍整页重录 | 只重画这块（与元素闭包求交）；小变化帧不可用的配置（例如关了自动 damage）退回显式脏区 |
| `onPaint` 画出边框盒 | 显示 | 被裁掉 |
| Lua 事件监听器里调扩展方法引起的重绘 | 整页 | 整页（监听器可能改任何地方，一律不归因） |

小变化帧的总开关是 `KUI_DISABLE_SMALL_CHANGE_PAINT_CLIP=1`（关掉后一律整页，声明与否没有区别）。

## 5. 自检清单

声明之前逐条核对，建议把结论写进扩展自己的文档：

- [ ] 列出 `onPaint` 读到的全部状态（文本、光标、选区、滚动、折叠、语法高亮、诊断、搜索高亮、补全、主题、焦点、输入法组合、闪烁相位、DPR……）。
- [ ] 对每一项，找出所有修改它的路径，确认每条路径之后都有 `scheduleRepaint` 或盖住新旧并集的 `invalidateRect`。
- [ ] 局部重绘的矩形与 `onPaint` 用的是同一套坐标（行号栏宽度、水平滚动、折行后的视觉行、输入法组合偏移都要算进去）。
- [ ] 后台线程只写队列，状态在 UI 线程上落地并请求重绘。
- [ ] `onPaint` 不调任何改 DOM / 属性的宿主函数，也不在里面请求重绘。
- [ ] 所有内容都在边框盒内（或接受被裁掉）。
- [ ] 焦点进出时画面会变的部分有重绘。

### 验证方法

KUI 提供校验模式：设 `KUI_VERIFY_SMALL_CHANGE_CLIP=1`，每个小变化帧上屏后宿主再整页重画一次逐像素比较，差异打印到 stderr
（`[KUI-SMALL-CHANGE-VERIFY] mismatch ...`，加 `KUI_VERIFY_SMALL_CHANGE_DUMP=<目录>` 存两幅画面）。声明约定后，把扩展的典型交互
（打字、光标移动、选区拖动、滚动、异步结果到达、焦点进出）在校验模式下跑一遍，**一次 mismatch 都不应出现**；出现即说明有漏报的重绘。

## 6. 违反约定的后果

宿主信任声明：漏报一次重绘，那块就停在旧画面上，直到别的原因重画它（校验模式能抓到）。拿不准时请不要声明——不声明只是性能上吃亏，不会画错。
