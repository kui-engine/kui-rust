# CSS 字体支持（@font-face / document.fonts）

KUI 支持 CSS `@font-face` 与 CSS Font Loading API 的实用子集（KUI-038, SDK 3.15.0+）。

## @font-face `src: local()` — 系统字体复用

`src: local("字体名")` 优先用系统已安装字体（零文件 IO），未安装再 fallback 到 `url()`：

```css
@font-face {
  font-family: "MyMono";
  src: local("Consolas"), url("fonts/mono.woff2") format("woff2");
}
```

先查系统有无 Consolas（Windows: DirectWrite/GDI），有则直接用（零文件 IO）；无则加载打包的 woff2。多源按 CSS 标准 fallback。

## `document.fonts` — CSS Font Loading API（子集）

```lua
-- 查某字体是否已注册（@font-face 加载完成）
if document.fonts.check("MyMono") then ... end
-- 主动加载 + 回调（KUI @font-face 同步加载，立即回调）
document.fonts.load("MyMono", function(ok) ... end)
-- 所有字体就绪回调（同步加载，立即触发）
document.fonts.ready(function() ... end)
-- 字体加载完成事件
document:addEventListener("loadingdone", function(e) ... end)
```

> **说明**：KUI 的 `@font-face` 字体为**同步加载**（首帧 layout 前完成），故 `ready`/`load` 立即就绪、`loadingdone` 在 @font-face 加载后即时派发。这替代了"轮询字体宽度变化"的兜底 hack。

## `font-display`

`font-display`（`auto`/`block`/`swap`/`fallback`/`optional`）descriptor 已解析存储。

> **当前限制（诚实标注）**：因 KUI 同步加载模型，字体在首帧 layout 前就绪、不存在 FOUT/FOIT，故 `font-display` 各值当前不产生可观察渲染差异。字段已存储，为未来异步/远程字体加载预留。

## `unicode-range`

`unicode-range` descriptor 已解析存储。

> **当前限制（诚实标注）**：码点级字体选择（按 Unicode 范围选不同字体）需 SkParagraph 的 per-run 字体匹配支持 `unicode-range`，当前 `TypefaceFontProvider` 按 `font-family` 注册、不含码点范围。已解析存储，码点级应用待 SkParagraph 集成。

---

> 字体渲染正确性（local() 实际命中系统字体、事件投递到 Lua 监听器）建议在真机环境复核 —— headless/无 GPU 环境无法完整验证字体光栅化与事件流。
