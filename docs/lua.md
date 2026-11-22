# KUI Lua Scripting - In-Depth Guide

> **Audience:** C++ developers using the KUI SDK who want to drive their UI with Lua scripts - either as inline `<script type="text/lua">` tags inside HTML, or as standalone `.lua` files loaded from C++.
>
> **Outcome:** After reading this document you will know how to enable the Lua engine, parse and execute `<script>` tags, manipulate the DOM (the largest single chapter of this guide), wire up event listeners, create custom HTML elements in pure Lua, use timers and Promises, and observe mutations / resizes / intersections - all from idiomatic Lua code with full DevTools breakpoint debugging.

**适用范围：** 本文描述当前仓库的 Lua 绑定，不保证所有旧版 SDK 都具有相同接口。Lua 由构建依赖提供，通过 [sol2](https://github.com/ThePhD/sol2) 绑定；CMake 的 `find_package(Lua QUIET)` 没有固定 Lua 小版本。请在实际加载的 SDK 中执行 `print(_VERSION)` 确认运行时版本，再选择对应版本的 Lua 手册。依赖目录中的头文件版本不能代替运行时验证。

> 本轮修订依据：当前绑定源码、现有回归测试定义及 MRG-61 审查。本文的 `checked` / `defaultChecked` 说明对应本次表单修复；不能据此认定旧版 SDK 已有这些行为，也不能把文档修订当成运行验证通过。

---

## Table of Contents

1. [Overview](#1-overview)
2. [Quick Start (4 lines)](#2-quick-start-4-lines)
3. [Architecture](#3-architecture)
4. [The DOM in Lua](#4-the-dom-in-lua) **(largest chapter)**
5. [Events](#5-events)
6. [Timers, Animation, Promises](#6-timers-animation-promises)
7. [CSS & Computed Styles](#7-css--computed-styles)
8. [Custom HTML Elements](#8-custom-html-elements)
9. [Observers (Mutation / Resize / Intersection)](#9-observers-mutation--resize--intersection)
10. [Data Binding & Reactive State](#10-data-binding--reactive-state)
11. [`<script>` Tag Execution](#11-script-tag-execution)
12. [C++ <-> Lua Interop](#12-c--lua-interop)
13. [Sandbox & Safety](#13-sandbox--safety)
14. [DevTools Debugger Integration](#14-devtools-debugger-integration)
15. [Examples Reference](#15-examples-reference)
16. [Troubleshooting](#16-troubleshooting)
17. [FAQ](#17-faq)
18. [See Also](#18-see-also)

---

## 1. Overview

### 1.1 What is KUI's Lua engine?

KUI ships with an integrated **Lua scripting engine** powered by [sol2](https://github.com/ThePhD/sol2). It lets you write HTML-driven UIs with Lua as the dynamic glue language. The host is a native C++ application, the DOM is KUI's own (Element / Document / Node), and the runtime is sandboxed Lua.

The engine is **opt-in** and **decoupled**: if you don't link `KUI_HAS_SCRIPTING`, your binary won't contain Lua at all. If you do, the same `kui.dll` you've already linked exposes the scripting API.

### 1.2 What can Lua do in KUI?

This is not a toy or a string-eval shim. The Lua surface is a serious DOM API:

| Capability | Examples |
|------------|----------|
| **DOM query** | `document:querySelector(".btn"):querySelectorAll("span")` |
| **DOM mutation** | `el.innerHTML = "<b>hi</b>"`, `parent:appendChild(child)`, `el:remove()` |
| **Attribute / class manipulation** | `el:setAttribute("data-x", "42")`, `el.classList:add("active")`, `el.dataset.userId = "7"` |
| **Inline styles** | `el.style:setProperty("color", "red")`, `el.style.cssText = "..."` |
| **Computed styles** | `local cs = document:getComputedStyle(el); print(cs.fontSize)` |
| **Event listeners** | `el:on("click", function(e) print(e.clientX, e.clientY) end)` |
| **Custom event dispatch** | `el:dispatchEvent("my-event", {detail = {x=1, y=2}})` |
| **Timers** | `setTimeout(fn, 100)`, `setInterval(fn, 16)`, `clearTimeout(id)` |
| **Animation frames** | `requestAnimationFrame(function(ts) ... end)` |
| **Promises** | `Promise.new(function(res, rej) ... end):andThen(...):catch(...)` |
| **JSON** | `JSON.parse(str)`, `JSON.stringify(table)` |
| **MutationObserver** | Observe attribute / childList / characterData changes |
| **ResizeObserver / IntersectionObserver** | Per-element resize and viewport visibility callbacks |
| **Custom HTML elements** | Define `class MyButton extends HTMLElement` analogue in Lua |
| **Window API** | `window.innerWidth()`、`window.innerHeight()`、`window.devicePixelRatio()`、`window.document()`、`window.setTitle("My App")`、`window.performance.now()`；这些是点调用函数 |
| **Window events** (3.5.0+) | `window:addEventListener("blur"/"focus"/"focusin"/"focusout"/"resize"/"close", fn[, {once=true}])` — app activation, keyboard focus, resize, cancelable close; 3.6.0+ adds `"movestart"`/`"move"`/`"moveend"` (window drag/move, Electron-style); 3.7.0+ adds `"dragregionpointerdown"` (app-region press, fires immediately without drag threshold) |
| **Canvas 2D / WebGL** | Draw and program shaders from Lua |
| **Media / Audio** | `<audio>` / `<video>` element control |
| **Two-way data binding** | Reactive state objects that auto-update DOM |
| **C++ <-> Lua interop** | Call C++ functions from Lua, register Lua callbacks for C++ |

### 1.3 Why Lua and not JavaScript?

- **Embeddable runtime** - Lua + sol2 的具体包体积取决于依赖版本和构建选项，请以实际 SDK 产物为准。
- **Embeddable by design** - no JIT, no syscalls, no filesystem unless we explicitly expose them.
- **Sandboxable** - per-engine memory cap (256 MB default), instruction-count cap (100M default), no `io`/`os`/`debug`/`package` libraries by default.
- **Fast startup** - the engine initialises in < 1 ms.
- **Easy C++ FFI** - sol2's templated binding is as ergonomic as it gets.
- **Coroutines first-class** - perfect for async / animation / generators.

If you absolutely need JavaScript, you can write a parallel `JsEngine : ScriptEngine` subclass; the `ScriptEngine` abstract base is language-agnostic by design.

---

## 2. Quick Start (4 lines)

```cpp
#include <kui/kui.h>
#include <kui/script/script_engine.h>

int main() {
    kui::Platform::init();
    auto window = kui::Window::create({.title = "Lua + KUI", .width = 800, .height = 600});

    auto* doc = window->document();
    doc->loadHTML(R"(
        <h1 id="greeting">Hello</h1>
        <button id="btn">Click me</button>
        <script type="text/lua">
            local btn = document:getElementById("btn")
            local h1  = document:getElementById("greeting")
            local n = 0
            btn:on("click", function(e)
                n = n + 1
                h1.textContent = "Clicked " .. n .. " times"
            end)
        </script>
    )");

    // <<< THESE 4 LINES ENABLE LUA >>>
    auto engine = kui::ScriptEngine::create();
    engine->bindDocument(doc);
    engine->executeScripts();       // runs all <script type="text/lua"> tags in document order
    window->setScriptEngine(engine.get());

    window->show();
    return kui::Platform::run();
}
```

Run it. Click the button. The `h1` text updates. That's all you need.

### What just happened?

- `ScriptEngine::create()` builds a `LuaEngine` (default Lua backend) and opens a sandboxed Lua state.
- `bindDocument(doc)` registers the global `document` variable and 30+ usertypes (Element / Node / Event / Timer / etc.) in the Lua environment.
- `executeScripts()` walks the DOM, finds every `<script type="text/lua">`, and runs them in document order.
- `window->setScriptEngine(engine.get())` lets DevTools attach to this engine (for breakpoint debugging) and routes script tick events.

You can also `evalFile("path/to/main.lua")` to run an external Lua file, or `eval("...")` to evaluate a code string. 这些调用必须在引擎和绑定文档所属的 UI 线程执行，且二者必须仍然存活；入口不会自动投递到 UI 线程。

---

## 3. Architecture

```
+------------------------------------------------------------+
|                Your C++ application                        |
|                                                            |
|   auto engine = kui::ScriptEngine::create();               |
|       |                                                    |
|       v                                                    |
|   +----------------+                                       |
|   | ScriptEngine   |  <-- abstract base class              |
|   |  (interface)   |                                       |
|   +-------+--------+                                       |
|           |                                                |
|           v                                                |
|   +----------------+    sol2     +----------------------+  |
|   |   LuaEngine    +-----------> |   sol::state         |  |
|   |  (impl)        |             |   (lua_State + RAII) |  |
|   +-------+--------+             +-----------+----------+  |
|           |                                  |             |
|           | bindDocument(doc)                |             |
|           v                                  v             |
|   +----------------+              +---------------------+  |
|   |  Document*     |              | Lua globals:        |  |
|   |  (DOM root)    | <----------- |   document          |  |
|   +-------+--------+              |   setTimeout        |  |
|           |                       |   Promise           |  |
|           |                       |   JSON              |  |
|           v                       |   window            |  |
|   +----------------+              |   ... (30+ types)   |  |
|   | Element / Node |              +---------------------+  |
|   | tree           |                                       |
|   +----------------+                                       |
+------------------------------------------------------------+
```

The bridge between Lua and your DOM is **bidirectional**:

- **C++ -> Lua:** `engine->eval("code")`, `engine->callFunction("fn", args)`, `engine->setGlobal("x", val)`.
- **Lua -> C++:** every DOM type (Element, Node, ...) is a sol2 usertype that calls real C++ methods. There is no marshalling layer in the hot path - Lua method calls are ~30 ns each.

### 3.1 Binding modules

The Lua surface is split across 19 source files for compile-time speed and clarity. Each registers a focused subset on first `bindDocument()`:

| Module | Surface |
|--------|---------|
| `lua_dom_bindings.cpp` | Document, Element, Node, NodeList, CSSStyle, ComputedStyle, classList, dataset, Stylesheet, CSSRule |
| `lua_event_bindings.cpp` | Event, MouseEvent, KeyboardEvent, addEventListener / dispatchEvent / preventDefault |
| `lua_timer.cpp` | setTimeout / setInterval / clearTimeout / clearInterval / requestAnimationFrame |
| `lua_custom_element.cpp` | Custom element definition + lifecycle callbacks |
| `lua_window_bindings.cpp` | window object, viewport size, devicePixelRatio, etc. |
| `lua_json.cpp` | JSON.parse / JSON.stringify (table <-> string) |
| `lua_canvas_bindings.cpp` | 2D Canvas drawing API |
| `lua_webgl_bindings.cpp` | WebGL 2.0 subset (43 KB) |
| `lua_media_bindings.cpp` | `<audio>` / `<video>` element control |
| `lua_audio_manager_bindings.cpp` | Higher-level audio mixer |
| `lua_component_bindings.cpp` | Component class system (Lua) |
| `lua_data_binding.cpp` | Reactive state objects |
| `lua_mutation_observer_bindings.cpp` | DOM mutation observation |
| `lua_resize_observer_bindings.cpp` | Element resize observation |
| `lua_intersection_observer_bindings.cpp` | Viewport intersection observation |
| `lua_debugger.cpp` | DevTools debugger backend (breakpoints / step / scope) |
| `interop_bridge.cpp` | High-performance C++ <-> Lua bidirectional calls |
| `extension_lua_marshal.cpp` | Extension DLL <-> Lua type marshalling |
| `lua_engine.cpp` | Lifecycle, eval, executeScripts, Promise bootstrap |

### 3.2 Threading model

Lua 引擎及其绑定的 DOM 采用单线程使用模型。宿主必须把 `eval`、`evalFile`、`callFunction`、`setGlobal`、计时器推进和 DOM 操作安排在同一个 UI 线程；这些入口不会自动切换线程。仅给 Lua 加互斥锁，也不能使 DOM、布局和窗口对象变成可跨线程访问。

`HttpClient::fetchAsync` 的完成回调运行在工作线程。回调只能整理响应数据，并通过宿主已经实现的 UI 投递机制或线程安全队列传递结果；**不得在该回调里直接调用 `engine->eval(...)`、`engine->callFunction(...)` 或操作 DOM**。由 UI 线程取出结果后，再调用 Lua。本文不假定存在某个特定的 KUI UI 投递 API。

投递时还必须处理寿命：工作线程和排队任务不应长期捕获未经保护的 `ScriptEngine*`、`Document*` 或 `Element*`。宿主应在 UI 消费端通过有效的所有权/弱引用机制或请求代次检查确认目标仍然存在；窗口关闭、文档替换或引擎销毁后丢弃过期结果。销毁时先停止接收新结果，并使已排队任务失效；不要把取消网络请求理解为“完成回调绝不会再执行”。

### 3.3 Object lifetime & ownership

- **查询结果是借用指针。** Lua 中已在 DOM 树内的 Element 由绑定返回裸 `Element*`；保存 Lua 变量不等于取得独立的节点强引用。不要在 `loadHTML()`、祖先 `innerHTML` 替换、节点移除或窗口/文档销毁后继续使用旧引用。需要延后处理时，由宿主先核对当前文档代次与寿命，再在 UI 线程查询节点。
- **新建节点有引擎临时所有权。** `document:createElement(...)` 等会把节点放进 `orphanNodes_`，挂入 DOM 时转交所有权；这个集合还会被清理，不能把它当作任意 Lua 裸引用的永久保活机制。
- **事件回调有引擎寿命保护。** 事件绑定使用 `weak_ptr<sol::function>` 等机制避免调用已释放的 Lua 函数；这不延长回调自行捕获的节点、文档或宿主对象的寿命。
- **部分移除路径有延迟释放。** DOM 变更会使用 pending-removal 等机制保护当前派发过程；这不保证任意保存的旧节点在后续回调或下一帧仍可访问，也不能把“本次没有崩溃”作为仍然有效的判断。

C++ 的 `RefPtr` 可在对象仍有效时取得节点本体的强引用，但节点所指向的文档、布局、样式和窗口有各自寿命。引用计数为原子操作也不代表 DOM 操作线程安全；线程安排见上一节。本节不承诺悬垂引用或跨线程误用会被统一捕获、抛错或给出诊断。

---

## 4. The DOM in Lua

Lua DOM 接口使用熟悉的 DOM 名称，但不承诺完整浏览器兼容。方法、属性、监听器句柄以及索引约定以各绑定为准：例如 `document:body()` 是方法，`el.checked` 是属性，NodeList 的 `item(i)` 和数字下标从 0 开始。

### 4.1 The `document` global

After `bindDocument()`, the global `document` is the live `kui::Document*`. Everything you do starts here.

#### Query methods

| Lua | Returns | Notes |
|-----|---------|-------|
| `document:getElementById("id")` | `Element` or `nil` | Fast hash lookup |
| `document:querySelector("selector")` | `Element` or `nil` | First match in document order |
| `document:querySelectorAll("selector")` | `NodeList` | Length via `#list` or `list:length()`; `list:item(i)` is 0-based; `list:forEach(fn)`; `list[i]` (0-based) also works |
| `document:getElementsByClassName("cls")` | `NodeList` | All elements with the class |
| `document:getElementsByTagName("div")` | `NodeList` | All elements with the tag |

```lua
local btn = document:getElementById("submit")
local items = document:querySelectorAll(".list-item")
print("Found " .. #items .. " list items")
items:forEach(function(el, i)
    print(i, el.tagName, el.textContent)
end)
```

#### Structure accessors

| Lua | Returns |
|-----|---------|
| `document:body()` | `<body>` element or nil |
| `document:head()` | `<head>` element or nil |
| `document:documentElement()` | `<html>` root element or nil |
| `document.title` (read/write) | document title string |
| `document.activeElement` | currently focused element or nil |

`document:body()`、`document:head()`、`document:documentElement()` 是方法，读取 `document.body` 等字段得到的是函数。`window` 是独立的 Lua table；当前 `window.innerWidth()`、`window.innerHeight()`、`window.devicePixelRatio()`、`window.document()` 同样需要调用，但使用点语法。前三者返回数字，最后一个返回绑定的 document。不要把这些函数写成浏览器式数值属性，也不要把这个约定套用到采用冒号的 `window:addEventListener(...)`。

```lua
document.title = "My App"
print("Currently focused: " .. (document.activeElement and document.activeElement.tagName or "none"))
```

#### Element creation

| Lua | Returns |
|-----|---------|
| `document:createElement("tag")` | New `Element`, detached |
| `document:createTextNode("text")` | New text `Node`, detached |
| `document:createDocumentFragment()` | New `DocumentFragment` (lightweight container) |
| `document:createComment("text")` | New comment node |

Created elements are kept alive by the engine until you attach them to the tree. They become tree-owned on `appendChild` / `insertBefore`:

```lua
local card = document:createElement("div")
card.className = "card"
card.innerHTML = "<h3>Title</h3><p>Body</p>"
document:body():appendChild(card)
```

#### Stylesheet helpers

| Lua | Effect |
|-----|--------|
| `document:addStylesheet(cssText)` | Append a new `<style>` block |
| `document.styleSheets` | Table of all `Stylesheet` objects |
| `document:getComputedStyle(el)` | Read-only `ComputedStyle` accessor (see Section 7) |

```lua
document:addStylesheet([[
    .card { padding: 16px; border: 1px solid #ccc; border-radius: 8px; }
    .card:hover { box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
]])
```

#### Lifecycle / events

| Lua | Effect |
|-----|--------|
| `document:invalidate()` | Force a re-layout + re-paint on next frame |
| `document:on("DOMContentLoaded", fn)` | Fires after `executeScripts()` completes |
| `document:addEventListener(type, fn[, opts])` | 注册文档监听器，可传 `{once=true}`；当前不返回监听器 ID，不能使用下文 Element 的单个 ID 移除方式 |
| `document:dispatchEvent(type[, opts])` | Fire a synthetic event |

```lua
document:on("DOMContentLoaded", function()
    print("DOM is ready, scripts have run")
end)
```

### 4.2 The `Element` usertype

Every element in the DOM is a Lua `Element` usertype. 必须按注册形式调用：方法用 `el:foo()`，属性用 `el.foo`；两种形式不能互换。

#### Identity & attributes

| Lua | Returns / sets |
|-----|----------------|
| `el.tagName` | Element tag (e.g. `"div"`) - read-only |
| `el.id` (rw) | `id` attribute |
| `el.className` (rw) | `class` attribute as space-separated string |
| `el:getAttribute(name)` | Attribute value or `""` |
| `el:setAttribute(name, value)` | Set / update attribute |
| `el:removeAttribute(name)` | Remove attribute |
| `el:hasAttribute(name)` | bool |

HTML 元素的 `el.tagName` 当前返回小写名称（例如 `"p"`、`"div"`），不同于浏览器
HTML DOM 常见的大写 `tagName`。从浏览器迁移 HTML 标签判断时，显式规范化后再比较：

```lua
local el = document:getElementById("d")
if el and string.lower(el.tagName) == "div" then
    print("HTML div")
end
```

此例仅用于 HTML 标签比较；不要据此改写 SVG、自定义元素或属性名称的大小写。

```lua
local input = document:getElementById("name")
input.id = "name"
input:setAttribute("placeholder", "Enter your name")
input:setAttribute("maxlength", "20")
print(input:getAttribute("type"))    -- "text"
```

#### Class manipulation

Two equivalent styles:

```lua
-- Legacy per-function API
el:addClass("active")
el:removeClass("hidden")
el:toggleClass("expanded")
print(el:hasClass("active"))         -- true

-- Modern classList API (DOMTokenList)
el.classList:add("active")
el.classList:remove("hidden")
el.classList:toggle("expanded")
el.classList:replace("a", "b")
print(el.classList:contains("active"))  -- true
print(el.classList.length)              -- 2
print(el.classList.value)               -- "active expanded"
```

#### Text & HTML content

| Lua | Behavior |
|-----|----------|
| `el.textContent` (rw) | Plain text. Reading returns concatenated descendant text. Writing replaces all children with a single text node. |
| `el.innerHTML` (rw) | HTML markup. Reading serialises children. Writing re-parses and replaces children. |

```lua
el.textContent = "Hello, world"
el.innerHTML = "<b>Hello</b>, <em>world</em>"
print(el.textContent)    -- "Hello, world"  (tags stripped)
print(el.innerHTML)      -- "<b>Hello</b>, <em>world</em>"
```

> **Safety note:** Both `textContent =` and `innerHTML =` use deferred removal of the old subtree, so any in-flight event handlers can still complete cleanly. You can safely call them from within an event listener.

#### Form controls: value, selection & cursor (`<input>` / `<textarea>`)

| Lua | Behavior |
|-----|----------|
| `el.value` (rw) | `input` / `select` / `textarea` 的当前值。checkbox/radio 使用 default/on 模式：缺少 `value` 属性时返回 `"on"`，显式空属性返回 `""`，写入会反射到该属性；普通文本输入的当前值与内容属性分离。Generic elements fall back to the attribute. |
| `el.checked` (rw) | input 的当前选中状态。脚本赋值不改 `checked` 内容属性，也不派发 `input` / `change`；radio 置 true 时取消同组当前选中项。 |
| `el.defaultChecked` (rw) | 反射 `checked` 内容属性是否存在。input 尚未经过当前态赋值或激活时，属性变化同步当前态；当前态已变脏后，默认态变化不覆盖它。 |
| `el.disabled` (rw) | Mapped to the `disabled` attribute |
| `el.selectionStart` (rw) | Selection start offset (input / textarea) |
| `el.selectionEnd` (rw) | Selection end offset; the caret sits at `selectionEnd` |
| `el:setSelectionRange(start, end)` | Set both ends at once; collapse with `start == end` to place the caret |
| `el.selectedText` (ro) | Currently selected substring |

```lua
local ta = document:getElementById("editor")
ta.value = "你好 world"
ta:setSelectionRange(0, 6)        -- selects 你好 (6 bytes of UTF-8)
print(ta.selectedText)            -- "你好"
ta:setSelectionRange(#ta.value, #ta.value)  -- caret to end
```

> **Offsets are UTF-8 byte offsets**, matching Lua's native string indexing
> (`#s`, `string.sub`). They are **not** UTF-16 code units as in web browsers.
> Since SDK 3.10.1, offsets that land inside a multi-byte sequence are
> automatically aligned down to the nearest codepoint boundary, so a
> miscomputed offset can never corrupt the buffer.

#### `dataset` (data-* attributes)

`el.dataset` is a proxy object that converts camelCase keys to kebab-case `data-*` attributes:

```lua
el.dataset.userId   = "42"           -- equivalent to el:setAttribute("data-user-id", "42")
print(el.dataset.userId)             -- "42"
print(el:getAttribute("data-user-id"))  -- "42"

el.dataset.role = "admin"
-- HTML now: <div data-user-id="42" data-role="admin">
```

This is the same convention as `HTMLElement.dataset` in browsers.

#### Tree navigation

| Property | Returns |
|----------|---------|
| `el.parentElement` | Parent `Element` or `nil` (skips non-element parents) |
| `el.parentNode` | Parent `Node` (any type) or `nil` |
| `el.firstElementChild` | First child that is an `Element` |
| `el.lastElementChild` | Last child element |
| `el.nextElementSibling` | Next sibling element |
| `el.previousElementSibling` | Previous sibling element |
| `el.childElementCount` | Number of child elements |
| `el.childNodes` | Lua table of all child nodes (mixed elements + text) |

```lua
-- Iterate element children only:
local child = root.firstElementChild
while child do
    print(child.tagName)
    child = child.nextElementSibling
end

-- Iterate all child nodes (elements + text):
for i, c in ipairs(root.childNodes) do
    print(i, c.nodeType, c.nodeName)
end
```

#### Subtree querying

```lua
local item = el:querySelector(".item")
local items = el:querySelectorAll(".item")
local closest = el:closest(".container")   -- walk up tree
print(el:matches(".widget"))               -- selector test
```

#### Mutation

| Lua | Effect |
|-----|--------|
| `parent:appendChild(child)` | Append; transfers ownership |
| `parent:insertBefore(newChild, refChild)` | Insert before refChild (or at end if refChild is nil) |
| `parent:replaceChild(newChild, oldChild)` | Replace |
| `parent:removeChild(child)` | Remove (returns the removed child) |
| `el:remove()` | Remove `el` from its parent |
| `el:cloneNode(deep)` | Shallow or deep clone |

```lua
-- Building a list:
local ul = document:createElement("ul")
for i = 1, 5 do
    local li = document:createElement("li")
    li.textContent = "Item " .. i
    ul:appendChild(li)
end
document:body():appendChild(ul)

-- Replacing the middle:
local items = ul:querySelectorAll("li")
local replacement = document:createElement("li")
replacement.innerHTML = "<b>Replaced</b>"
ul:replaceChild(replacement, items:item(2))
```

### 4.3 The `Node` base type

`Element`, text nodes, comments, and document fragments all derive from `Node`. The base properties are usually transparent (an `Element` "is a" `Node`), but for text/comment manipulation you may need the lower-level API:

| Property | Meaning |
|----------|---------|
| `node.nodeType` | 1 = ELEMENT, 3 = TEXT, 8 = COMMENT, 11 = FRAGMENT |
| `node.nodeName` | Tag name for elements; `"#text"`, `"#comment"` for others |
| `node.textContent` (rw) | Same semantics as `Element.textContent` |
| `node.parentNode` / `node.firstChild` / `node.nextSibling` / etc. | Like Element but covers all node types |
| `node.ownerDocument` | The Document the node belongs to |
| `node.isConnected` | true if reachable from document root |
| `node:hasChildNodes()` | bool |
| `node:contains(other)` | true if `other` is in `node`'s subtree |
| `node:getRootNode()` | Walks up to the topmost ancestor |

```lua
for i, child in ipairs(el.childNodes) do
    if child.nodeType == 3 then           -- text node
        print("Text:", child.textContent)
    elseif child.nodeType == 1 then       -- element
        print("Element:", child.nodeName)
    end
end
```

### 4.4 `NodeList`

Returned by `querySelectorAll` and the `getElements*` methods. Iterable two ways:

```lua
local items = document:querySelectorAll(".item")

-- Form 1: explicit indexing (0-based, like JS)
for i = 0, #items - 1 do
    print(items:item(i).textContent)
end

-- Form 2: forEach
items:forEach(function(el, i)
    print(i, el.textContent)        -- i is 0-based to match JS
end)

-- Form 3: numeric subscript (0-based)
print(items[0].textContent)
print(items[#items - 1].textContent)
```

The `#list` operator returns the length (same as `list:length()`). `list.length` 是方法本身，不是长度数值。

> **Note:** `NodeList` is a *snapshot* at query time. Subsequent DOM mutations do not update it. If you need a live collection, re-query.

### 4.5 Putting it together: a small CRUD demo

```lua
-- HTML markup expected:
-- <ul id="todos"></ul>
-- <input id="new-todo" type="text" placeholder="What to do?" />
-- <button id="add">Add</button>

local list  = document:getElementById("todos")
local input = document:getElementById("new-todo")
local btn   = document:getElementById("add")

local function addTodo(text)
    if text == "" then return end
    local li = document:createElement("li")
    li.className = "todo-item"
    li.innerHTML = '<span class="text"></span> <button class="del">x</button>'
    li:querySelector(".text").textContent = text
    li:querySelector(".del"):on("click", function()
        li:remove()
    end)
    list:appendChild(li)
end

btn:on("click", function()
    addTodo(input.value)
    input.value = ""
end)

input:on("keydown", function(e)
    if e.key == "Enter" then
        btn:dispatchEvent("click")
    end
end)
```

That's a fully functional to-do list in 20 lines of Lua.

### 4.6 DOM cheat sheet

A copy-paste reference for the most common operations:

```lua
-- ============================
-- Query
-- ============================
local el  = document:getElementById("x")
local el  = document:querySelector(".cls")
local lst = document:querySelectorAll("div.cls")

-- ============================
-- Attributes & classes
-- ============================
el:setAttribute("name", "value")
el:removeAttribute("name")
el.id        = "new-id"
el.className = "btn primary"

el.classList:add("a")
el.classList:remove("b")
el.classList:toggle("c")
print(el.classList:contains("a"))

el.dataset.userId = "7"

-- ============================
-- Text & HTML
-- ============================
el.textContent = "plain text"
el.innerHTML   = "<b>html</b>"

-- ============================
-- Inline styles
-- ============================
el.style:setProperty("color", "red")
el.style:setProperty("font-size", "16px", "important")
el.style.cssText = "color:red;font-size:16px"
el.style:removeProperty("color")

-- ============================
-- Tree
-- ============================
parent:appendChild(child)
parent:insertBefore(newChild, refChild)
parent:replaceChild(newChild, oldChild)
parent:removeChild(child)
child:remove()
local clone = el:cloneNode(true)

-- ============================
-- Events
-- ============================
local id = el:on("click", function(e) ... end)
el:on("input", function(e) ... end, {once = true})
el:removeEventListener("click", id)
el:dispatchEvent("custom-event", {detail = {foo = "bar"}})
```

---

## 5. Events

KUI Lua's event system is functionally a subset of the standard DOM Event interface. Listeners get a single `Event` (or subclass) argument.

### 5.1 Adding & removing listeners

```lua
-- Standard form: returns a numeric listener ID
local id = el:addEventListener("click", function(e)
    print("clicked at", e.clientX, e.clientY)
end)

-- Alias (recommended for brevity):
local id = el:on("click", function(e) ... end)

-- Once: auto-remove after first dispatch
el:on("click", function(e) print("fired once") end, {once = true})

-- Removing one listener:
el:removeEventListener("click", id)

-- Removing all listeners of a type:
el:removeEventListeners("click")
```

Element 的 `addEventListener` / `on` 返回数字监听器 ID。移除单个监听器时，必须在同一个元素上传入原事件类型和该 ID：`el:removeEventListener(type, id)`，不能传回调函数。每次注册都有自己的 ID；ID 不是跨对象通用的句柄。

`window:addEventListener(...)` / `window:removeEventListener(type, id)` 也使用数字 ID；没有关联原生窗口时，注册返回 0。`document:addEventListener(...)` / `document:on(...)` 当前不返回 ID，且没有对应的 `document:removeEventListener(type, id)` 绑定。不要把 Element 的移除示例直接用于 document。

### 5.2 Event types

| Type | Subclass | Useful fields |
|------|----------|---------------|
| `"click"` / `"mousedown"` / `"mouseup"` / `"mousemove"` | `MouseEvent` | `clientX`, `clientY`, `button`, `ctrlKey`, `shiftKey`, `altKey`, `metaKey` |
| `"keydown"` / `"keyup"` / `"keypress"` | `KeyboardEvent` | `key`, `code`, `keyCode`, `ctrlKey`, `shiftKey`, `altKey`, `metaKey`, `repeat` |
| `"input"` / `"change"` | `InputEvent` | `data`, `inputType` |
| `"focus"` / `"blur"` | `FocusEvent` | `relatedTarget` |
| `"pointerdown"` / `"pointermove"` / `"pointerup"` / `"pointercancel"` | `PointerEvent` | `pointerId`, `pressure`, `tiltX`, `tiltY`, `pointerType`, plus all MouseEvent fields |
| `"wheel"` | `WheelEvent` | `deltaX`, `deltaY`, `deltaZ`, `deltaMode` |
| `"resize"` | `Event` | (target dimensions via `target` and getBoundingClientRect) |
| `"contextmenu"` (3.11.0+) | `MouseEvent` | `button` (right); fires on right-click **and** keyboard (Shift+F10 / menu key); the built-in context menu (Cut / Copy / Paste / Select All) is **off by default** since 3.61.8 — enable it with `<html context-menu>` or `EventDispatcher::setDefaultContextMenuEnabled(true)`; when enabled, `preventDefault()` suppresses it. With the inspector on, right-click still offers "Inspect Element" |
| `"menu-open"` / `"menu-close"` (3.61.8+) | `MouseEvent` / `Event` | dispatched on a `<kui-context-menu>` when it opens / closes; `menu-open` is cancelable and its `relatedTarget` is the right-clicked element (see components.md §4.5) |
| `"dblclick"` | `MouseEvent` | `detail` (=2), plus all MouseEvent fields |
| `"mouseenter"` / `"mouseleave"` (non-bubbling) / `"mouseover"` / `"mouseout"` (bubbling) | `MouseEvent` | `relatedTarget` |
| `"dragenter"` / `"dragover"` / `"dragleave"` / `"drop"` (3.11.0+) | `DragEvent` (extends MouseEvent) | `dataTransfer` (`.files`, `:getData(fmt)`, `.types`, `.dropEffect`), plus all MouseEvent fields — see §5.6 |
| `"compositionstart"` / `"compositionupdate"` / `"compositionend"` (3.11.0+) | `InputEvent` | `data` (the composing string; the committed string on `compositionend`); observe-only (not cancelable) — for IME-aware input on `<input>` / `<textarea>` / editable elements |
| `"copy"` / `"cut"` / `"paste"` (3.11.0+) | `InputEvent` (cancelable) | `data` (selection text for copy/cut; clipboard text for paste); `preventDefault()` suppresses the default clipboard action (e.g. to sanitize pasted text) |
| custom (e.g. `"my-event"`) | `Event` or `CustomEvent` | `detail` (from `dispatchEvent`) |

All events have:

- `e.type` - event name
- `e.target` - the element on which the listener was registered (more precisely, the dispatch target)
- `e.currentTarget` - same as `target` in KUI's simplified bubbling model
- `e.timeStamp` - high-resolution timestamp
- `e:preventDefault()`
- `e:stopPropagation()`
- `e.defaultPrevented` (bool)

### 5.3 Dispatching custom events

```lua
-- Simple dispatch
el:dispatchEvent("my-event")

-- With payload (accessed via the global `__kui_event_detail` in handler, or e.detail)
el:dispatchEvent("status-changed", {detail = {oldStatus = "draft", newStatus = "published"}})

-- Listening:
el:on("status-changed", function(e)
    print(e.detail.oldStatus, "->", e.detail.newStatus)
end)
```

### 5.4 Document-level listeners

```lua
document:on("keydown", function(e)
    if e.key == "Escape" then
        modal:remove()
    end
end)

document:on("DOMContentLoaded", function()
    print("ready")
end)
```

Document-level listeners are also registered on the `<body>` element internally, so OS-dispatched keyboard / focus events still fire.

### 5.5 Pointer capture (drag-and-drop building block)

```lua
local handle = document:querySelector(".drag-handle")
local dragging = false

handle:on("pointerdown", function(e)
    dragging = true
    handle:setPointerCapture(e.pointerId)
end)

handle:on("pointermove", function(e)
    if not dragging then return end
    -- e.clientX / e.clientY are valid even if cursor leaves the element
    -- because of setPointerCapture
end)

handle:on("pointerup", function(e)
    dragging = false
    handle:releasePointerCapture(e.pointerId)  -- optional: see note below
end)
```

While a pointer is captured, `pointermove` / `pointerup` are delivered to the
capturing element even after the cursor leaves it (and even when the button is
released outside the window) — `e.clientX` / `e.clientY` carry the real cursor
position throughout. Capture is **released implicitly on `pointerup`** (which
fires `lostpointercapture`), so the explicit `releasePointerCapture` above is
optional; calling it again is a safe no-op. Each platform event invokes a
document/element listener exactly once.

### 5.6 External file drop (3.11.0+)

Files dragged from the OS file manager (Explorer / Finder) onto the window
produce standard DOM drag events. Listen on any element (or `document`) and
read the dropped paths from `e.dataTransfer.files`:

```lua
-- Highlight a drop zone while files hover over it.
local zone = document:getElementById("drop-zone")

zone:on("dragover", function(e)
    e:preventDefault()           -- required: marks the element as a drop target
    zone:setAttribute("class", "drop-zone drag-over")
end)

zone:on("dragleave", function(e)
    zone:setAttribute("class", "drop-zone")
end)

zone:on("drop", function(e)
    zone:setAttribute("class", "drop-zone")
    for _, path in ipairs(e.dataTransfer.files) do   -- absolute paths, UTF-8
        print("dropped: " .. path)
        -- e.clientX / e.clientY give the drop position;
        -- use document:elementFromPoint(x, y) to route to a sub-target.
    end
end)
```

`e.dataTransfer` (a `DataTransfer`) also exposes `:getData(format)`,
`.types`, and `.dropEffect`. The event sequence is
`dragenter → dragover (repeated while hovering) → drop → dragleave`.

> Platform note: on Windows the primary path is an OLE `IDropTarget`
> (live `dragover` while hovering, for drop-zone highlighting). Processes
> running **elevated** fall back to `WM_DROPFILES` (a synthesized
> enter→drop→leave at release time) because UIPI blocks cross-integrity OLE
> drag-and-drop; the `files` payload is identical on both paths.

---

## 6. Timers, Animation, Promises

### 6.1 `setTimeout` / `setInterval`

```lua
local timeoutId = setTimeout(function()
    print("100ms later")
end, 100)

local intervalId = setInterval(function()
    print("tick")
end, 1000)

-- Cancel:
clearTimeout(timeoutId)
clearInterval(intervalId)
```

Defaults: `setTimeout` with no delay = 0 ms; `setInterval` with no delay = 16 ms (~60 FPS).

The engine ticks timers inside `Document::updateIfNeeded()` on every animation frame. If you have no pending timers and no rAF requests, the animation timer is paused (zero CPU cost).

### 6.2 `requestAnimationFrame`

```lua
local function loop(timestamp)
    -- timestamp is milliseconds since the engine started
    local el = document:getElementById("ball")
    el.style:setProperty("left", math.floor(timestamp / 10) % 400 .. "px")
    requestAnimationFrame(loop)
end

requestAnimationFrame(loop)
```

The callback receives the current high-resolution timestamp. As long as you re-`requestAnimationFrame`, the loop continues. To stop: simply don't re-request.

### 6.3 Promises (built-in, pure Lua)

The engine injects a Promise implementation at bind time. The API is JavaScript-Promise-shaped:

```lua
local p = Promise.new(function(resolve, reject)
    setTimeout(function()
        resolve("done")
    end, 500)
end)

p:andThen(function(value)
    print("got:", value)        -- "got: done"
    return value .. "!"
end):andThen(function(value2)
    print(value2)               -- "done!"
end):catch(function(err)
    print("error:", err)
end)

-- Combinators:
Promise.all({p1, p2, p3}):andThen(function(results)
    print(table.concat(results, ", "))
end)

local resolved = Promise.resolve(42)
local rejected = Promise.reject("oops")
```

Promise scheduling uses `setTimeout(fn, 0)` so callbacks are deferred to the next frame, matching the browser microtask convention (close enough for UI purposes).

---

## 7. CSS & Computed Styles

### 7.1 Inline styles (`el.style`)

```lua
-- Set / read individual properties
el.style:setProperty("color", "red")
el.style:setProperty("font-size", "16px")
el.style:setProperty("z-index", "1000", "important")

print(el.style:getPropertyValue("color"))  -- "red"
print(el.style:getPropertyPriority("z-index"))  -- "important"

el.style:removeProperty("color")

-- Bulk read/write
print(el.style.cssText)
el.style.cssText = "color: blue; padding: 8px"
```

Properties take effect on the next layout pass. Multiple property changes in the same tick coalesce into a single re-layout.

### 7.2 Computed styles

```lua
local cs = document:getComputedStyle(el)

-- Two equivalent forms:
print(cs:getPropertyValue("font-size"))    -- "16px"
print(cs.fontSize)                          -- "16px"  (camelCase shortcut via __index)
print(cs.fontFamily)                        -- "Arial"
print(cs["background-color"])              -- "rgb(0, 0, 0)"
```

The `cs.camelCase` form auto-converts to `kebab-case` for CSS property lookup, so both work interchangeably.

`getComputedStyle` triggers a style resolution if needed; subsequent reads in the same frame are cached.

### 7.3 Adding stylesheets at runtime

```lua
document:addStylesheet([[
    body { font-family: system-ui; }
    .card {
        padding: 16px;
        border-radius: 8px;
        background: var(--card-bg, #fff);
    }
    @media (max-width: 600px) {
        .card { padding: 8px; }
    }
]])
```

The new sheet is added at the end of the cascade, so its rules win ties with same specificity. Re-layout is automatic.

### 7.4 Reading & editing existing stylesheets

```lua
for i, sheet in ipairs(document.styleSheets) do
    print("Sheet " .. i .. ": " .. sheet.ruleCount .. " rules")
    for j, rule in ipairs(sheet:cssRules()) do
        if rule.type == 1 then   -- CSSRuleType::Style
            print("  " .. rule.selectorText .. " { " .. rule.cssText .. " }")
        end
    end
end

-- Add a rule
sheet:insertRule(".active { background: yellow; }")

-- Disable / re-enable a sheet
sheet.disabled = true
```

`insertRule` / `deleteRule` / `disabled` and writes through `rule.style` (`setProperty`, `removeProperty`, `cssText`, `rule.style.color = ...`) take effect on the next frame, exactly like `document:addStylesheet(...)`: the rule index is rebuilt, the whole tree is restyled and a repaint is requested. A disabled sheet takes no part in matching. `insertRule("@keyframes ...")` registers the animation so `animation-name` can refer to it. (In 3.58.0 and earlier these edits only took effect once something else triggered a restyle, and `disabled` had no effect at all.)

C++ code gets the same behaviour: mutating a `kui::Stylesheet` that has been added to a document (`insertRule` / `deleteRule` / `appendRule`, `setDisabled` / `setOrigin`, `CSSStyleRule::setStyle` / `setSelectorText`, or writes to a rule's declaration block, including rules nested in `@media` / `@supports` / `@layer` / `@container`) notifies every document that holds the sheet. (In 3.58.0 and earlier the host had to call `invalidateRuleIndex()`, `markSubtreeNeedsStyle()` and `scheduleRepaint()` itself.) Setters that are inline in the public headers — `setSelectorList`, `addRule` / `addNestedRule` on grouping rules, `setMediaText` — do not notify; to change those after the sheet was added, `removeStylesheet` it and `addStylesheet` it again.

---

## 8. Custom HTML Elements

Define new HTML tags in pure Lua and use them in your markup. This is the equivalent of Web Components / Custom Elements, scoped to KUI's renderer.

### 8.1 Defining a custom element

```lua
defineCustomElement("my-greeting", {
    -- Called when an instance is created (constructor)
    connectedCallback = function(self)
        self.innerHTML = '<span>Hello, <span class="name"></span>!</span>'
        local nameSpan = self:querySelector(".name")
        local name = self:getAttribute("name")
        nameSpan.textContent = name ~= "" and name or "world"
    end,

    -- Called when an instance is removed from the DOM
    disconnectedCallback = function(self)
        print("greeting removed")
    end,

    -- Observed attribute changes
    observedAttributes = {"name"},
    attributeChangedCallback = function(self, attrName, oldValue, newValue)
        if attrName == "name" then
            self:querySelector(".name").textContent = newValue
        end
    end
})
```

Now any `<my-greeting name="Alice">` in your HTML will be upgraded. Mutating its `name` attribute (from Lua or DevTools) re-renders.

### 8.2 Lifecycle order

1. HTML parser encounters `<my-greeting>` and creates a plain element.
2. Engine spots the registered tag, attaches the Lua class table, and calls `connectedCallback(self)`.
3. Subsequent `setAttribute("name", "...")` calls fire `attributeChangedCallback(self, name, old, new)`.
4. On removal from the tree, `disconnectedCallback(self)` fires (once).

### 8.3 Communicating with C++ from custom elements

Inside `connectedCallback`, `self` is the underlying `Element*`, so any of the DOM methods from Section 4 work normally. Set a `data-*` attribute or fire a `dispatchEvent("...")` to communicate back to other listeners.

---

## 9. Observers (Mutation / Resize / Intersection)

### 9.1 MutationObserver

Watch for DOM changes (attribute / childList / characterData) on a subtree:

```lua
local observer = MutationObserver.new(function(records)
    for _, rec in ipairs(records) do
        print(rec.type, rec.target.tagName, rec.attributeName)
        -- rec.addedNodes / rec.removedNodes are tables of Node
    end
end)

observer:observe(document:body(), {
    childList     = true,
    attributes    = true,
    subtree       = true,
    attributeOldValue = true
})

-- Later:
observer:disconnect()
```

Useful for: undo / redo systems, diff tracking, lazy-loading triggered by DOM injection.

### 9.2 ResizeObserver

```lua
local ro = ResizeObserver.new(function(entries)
    for _, entry in ipairs(entries) do
        local rect = entry.contentRect
        print(entry.target.id, "is now", rect.width, "x", rect.height)
    end
end)

ro:observe(document:getElementById("chart"))

-- Later:
ro:unobserve(document:getElementById("chart"))
ro:disconnect()
```

The callback fires on the same frame as layout, so you can read final sizes without flicker.

### 9.3 IntersectionObserver

Detect when an element enters / exits the viewport (lazy-load, infinite-scroll, virtualisation):

```lua
local io = IntersectionObserver.new(function(entries)
    for _, entry in ipairs(entries) do
        if entry.isIntersecting then
            entry.target.classList:add("visible")
            -- Load image / data...
        end
    end
end, {
    root       = nil,        -- nil = viewport
    rootMargin = "100px",    -- pre-load 100px before they enter
    threshold  = 0.1         -- fire when 10% visible
})

document:querySelectorAll(".lazy-image"):forEach(function(el)
    io:observe(el)
end)
```

---

## 10. Data Binding & Reactive State

KUI's data-binding layer is a lightweight reactive-state system reminiscent of Vue 3 or Solid.js, optimised for the KUI DOM.

### 10.1 Reactive state objects

```lua
local state = reactive({
    count = 0,
    user  = { name = "Alice", age = 30 }
})

-- Subscribe to changes
watch(state, "count", function(newVal, oldVal)
    print("count: " .. oldVal .. " -> " .. newVal)
end)

state.count = 1            -- triggers the watcher
state.user.name = "Bob"    -- nested mutations also tracked
```

### 10.2 Binding to the DOM

```lua
local model = reactive({
    title = "Hello",
    items = {"apple", "banana", "cherry"}
})

-- Two-way bind an input to model.title
bindInput(document:getElementById("title-input"), model, "title")

-- One-way bind: re-render a list when items change
bindList(document:getElementById("item-list"), model, "items", function(item)
    local li = document:createElement("li")
    li.textContent = item
    return li
end)

-- Mutate the model; DOM updates automatically
table.insert(model.items, "date")
```

### 10.3 Computed properties

```lua
local model = reactive({a = 1, b = 2})
local sum = computed(function() return model.a + model.b end)

print(sum())  -- 3
model.a = 10
print(sum())  -- 12
```

`computed` returns a function that caches the result; it re-evaluates only when one of its dependencies changes.

---

## 11. `<script>` Tag Execution

### 11.1 How `<script>` tags are run

When you call `engine:executeScripts()`, the engine walks the entire DOM in document order and runs each `<script>` it finds.

```html
<head>
    <script type="text/lua">
        document:on("DOMContentLoaded", function()
            print("Will fire after all scripts run")
        end)
    </script>
</head>
<body>
    <h1 id="title">Hello</h1>
    <script src="main.lua"></script>     <!-- external -->
    <script type="text/lua">
        document:getElementById("title").textContent = "Updated"
    </script>
</body>
```

### 11.2 `type` attribute filtering

The engine only runs scripts whose `type` is empty, `text/lua`, or `application/lua`. Other types (`text/javascript`, `module`, ...) are skipped silently.

```html
<script type="text/lua">  ... </script>   <!-- runs -->
<script>                  ... </script>   <!-- runs (no type defaults to Lua) -->
<script type="text/javascript">...</script>   <!-- ignored -->
```

### 11.3 External scripts (`src`)

```html
<script src="./app.lua"></script>
```

`src` is resolved relative to the current document. The engine calls `evalFile(path)`. If both `src` and inline content are present, `src` wins (per HTML spec).

### 11.4 `DOMContentLoaded`

After all scripts have run, the engine fires a `DOMContentLoaded` event on the document. Use it for initialisation logic that should run last:

```lua
document:on("DOMContentLoaded", function()
    -- All other scripts have finished; the DOM is fully populated.
    initializeApp()
end)
```

### 11.5 Hot-loading scripts at runtime

After initial execution, you can run more code at any time:

```cpp
// from C++
engine->eval("print('hello from C++')", "<runtime>");
engine->evalFile("plugin.lua");
```

Or from Lua itself:

```lua
-- (not recommended; usually defineCustomElement is cleaner)
loadstring(codeString)()
```

---

## 12. C++ <-> Lua Interop

### 12.1 Calling Lua from C++

```cpp
// String eval
auto result = engine->eval("return 2 + 3", "<expr>");
if (result.success) {
    // result.value holds a ScriptValue (int / double / string / table / ...)
    std::cout << result.value.toDouble() << std::endl;  // 5
}

// Call a named Lua function
engine->setGlobal("multiplier", kui::ScriptValue::makeDouble(3.0));
engine->eval("function timesM(x) return x * multiplier end");

std::vector<kui::ScriptValue> args = { kui::ScriptValue::makeDouble(7.0) };
auto r = engine->callFunction("timesM", args);
std::cout << r.value.toDouble() << std::endl;   // 21
```

### 12.2 Calling C++ from Lua

Register a C++ callable as a global Lua function:

```cpp
engine->registerFunction("nativeLog", [](const std::vector<kui::ScriptValue>& args) {
    if (args.empty()) return kui::ScriptValue{};
    std::string msg = args[0].toString();
    KUI_LOG_INFO("MyApp", msg);
    return kui::ScriptValue::makeBool(true);
});
```

Then from Lua:

```lua
nativeLog("This message goes to KUI_LOG_INFO")
```

### 12.3 `setGlobal` / `getGlobal`

For simple value exchange:

```cpp
engine->setGlobal("appVersion", kui::ScriptValue::makeString("3.1.0"));

// Lua side:
//   print(appVersion)   -- "3.1.0"

auto userCount = engine->getGlobal("userCount").toInt();
```

### 12.4 `print` redirection

By default, `print(...)` in Lua writes to `stdout`. To capture into your app's logging:

```cpp
engine->setPrintHandler([](const std::string& msg) {
    KUI_LOG_INFO("Lua", msg);   // also lights up DevTools console
});
```

When DevTools is attached, `print` output also appears in the Console panel (via `KUI_LOG_INFO`).

### 12.5 InteropBridge (high-performance bidirectional calls)

For hot-path code that calls back and forth thousands of times per frame, the `InteropBridge` provides typed registered methods bypassing the generic ScriptValue marshalling:

```cpp
auto* bridge = engine->interopBridge();
bridge->registerMethod("computePhysics", [](float dt) -> float {
    // ~30 ns per call from Lua
    return mySim.step(dt);
});
```

```lua
local dt = 1/60
local result = kuiInterop:computePhysics(dt)
```

This is overkill for normal UI logic; reach for it only when profiling shows generic marshalling on a hot path.

### 12.6 Native functions that change what is painted

Functions registered with `registerFunction` or `InteropBridge::registerMethod` run inside Lua callbacks (timers, `requestAnimationFrame`, event listeners). KUI does **not** repaint the whole window just because a Lua callback ran: a frame only repaints what demonstrably changed. Changes made through the DOM / CSSOM / form / canvas APIs are tracked automatically — set attributes, styles, text, values, scroll positions, draw on a canvas, and the right region is repainted.

If a native function changes something that is painted **without** going through those APIs — state read by a custom element's paint callback, an extension's internal state, a pixel buffer it hands to an element — it must request a repaint itself:

```cpp
bridge->registerMethod("setGaugeValue", [doc](float v) {
    g_gauge.value = v;          // read by the <x-gauge> paint callback
    doc->scheduleRepaint();     // required: KUI cannot see this write
});
```

The same rule already applies to any host C++ code that runs outside Lua. Extensions declare it through the repaint contract (see `extension_repaint_contract.md`). To restore the old "any Lua timer / rAF callback repaints the whole window" behaviour while diagnosing a stale-paint problem, set `KUI_DISABLE_SMALL_CHANGE_SCRIPT_TIMERS=1`.

---

## 13. Sandbox & Safety

The Lua engine is intentionally sandboxed.

### 13.1 Disabled stdlib modules

The following are **not loaded** by default:

| Module | Why disabled |
|--------|--------------|
| `io` | Filesystem access |
| `os` | Process control, time-of-day, exit |
| `debug` | Stack walking, sethook (used internally) |
| `package` | Module loading via `require` |

If you genuinely need one of them in trusted scripts, set the global yourself from C++:

```cpp
auto& L = static_cast<kui::LuaEngine*>(engine.get())->lua();
L.open_libraries(sol::lib::os);   // careful!
```

### 13.2 Memory cap

Default: **256 MB per engine**. The custom allocator refuses allocations beyond this. Lua receives an `out of memory` error and the offending coroutine throws.

You can adjust this before `bindDocument`:

```cpp
// Not currently exposed via the public ScriptEngine interface; use LuaEngine directly.
```

### 13.3 Instruction-count cap

Default: **100M Lua bytecode instructions per `eval()` call**. This breaks accidental infinite loops:

```lua
-- This will eventually throw "instruction limit exceeded"
while true do end
```

The hook is re-installed per eval, so the debugger's hook (if attached) is preserved correctly.

### 13.4 Sandboxed `pcall`

All Lua callbacks (event listeners, timers) are wrapped in `sol::protected_function`. An error inside a listener:

- does NOT crash the application,
- does NOT abort the event dispatch,
- is reported via the engine's error handler with file + line + stack trace.

```cpp
engine->setErrorHandler([](const String& err, const String& src, int line) {
    KUI_LOG_ERROR("Lua", src + ":" + std::to_string(line) + " - " + err);
});
```

---

## 14. DevTools Debugger Integration

When you wire the engine into a window with `window->setScriptEngine(engine.get())` and start DevTools, the DevTools **Sources** panel shows every loaded Lua script - inline `<script>` blocks and external `.lua` files alike.

### Supported features

- **Breakpoints** - click any gutter line
- **Conditional breakpoints** - right-click a breakpoint, add expression
- **Logpoints** - log a message instead of pausing
- **Step over / into / out**
- **Call stack** inspection with scope variables
- **Watch expressions**
- **Pause on exception** (caught / uncaught toggles)
- **Pause on event listener** (e.g., pause on every `click`)
- **Lua REPL** in the Console tab

See [`devtools.md`](./devtools.md) Section 10 for full setup details.

### Why this works

Internally, `LuaEngine::debugger()` returns a `LuaDebugger` (single instance per engine, lazily created), which installs a `lua_sethook` line callback. DevTools issues `Debugger.*` commands (see [`devtools.md`](./devtools.md) Section 5.5) which the debugger services on the UI thread.

---

## 15. Examples Reference

The SDK ships five runnable Lua + DOM example programs under `examples/native/script/`:

| Example | Topic | Key APIs demonstrated |
|---------|-------|-----------------------|
| `01_lua_hello` | Basics | `bindDocument`, `eval`, `print` |
| `02_lua_dom` | DOM manipulation | `getElementById`, `createElement`, `innerHTML`, `appendChild` |
| `03_lua_events` | Event handling | `addEventListener`, `dispatchEvent`, `preventDefault` |
| `04_lua_animation` | Animation | `requestAnimationFrame`, `style.cssText`, computed styles |
| `05_lua_data` | Data binding | `reactive`, `watch`, `computed`, `bindList` |

To build & run example 02:

```bash
cmake -S sdk/cpp/examples/native/script/02_lua_dom -B build/lua02 \
      -DCMAKE_PREFIX_PATH=<path/to/sdk/cpp>
cmake --build build/lua02 --config Release
build/lua02/Release/02_lua_dom.exe
```

Plus the larger `examples/tank_game_lua/` showcases a complete game loop driven entirely from Lua with C++ providing only the entry point.

---

## 16. Troubleshooting

### 16.1 `attempt to index a nil value (global 'document')`

You forgot to call `engine->bindDocument(doc)` before running scripts. The `document` global is registered by `bindDocument`.

### 16.2 `attempt to call a nil value (method 'on')`

You called `el.on(...)` (dot) instead of `el:on(...)` (colon). The colon syntax passes `el` as the first argument; dot does not. Lua method calls always need `:`.

### 16.3 My event listener doesn't fire

Three causes:

1. The element was removed and re-added (created via `innerHTML =`). Listeners attached to the old DOM are gone; re-attach on the new elements.
2. The listener was registered before the element existed (script ran before `<element>` was parsed). Wrap in `document:on("DOMContentLoaded", function() ... end)`.
3. Wrong event name. KUI uses standard DOM names (`click`, `keydown`, `pointermove`) - not `onclick` or `onClick`.

### 16.4 `instruction limit exceeded`

Your script ran 100M Lua instructions in one `eval()`. Likely an unintended infinite loop. The hook fires synchronously inside the offending coroutine; check the stack trace.

### 16.5 Out of memory

The 256 MB cap was hit. Investigate large table allocations or leaks (e.g. closures capturing huge objects).

### 16.6 DevTools Console says "scripting unavailable"

`window->setScriptEngine(engine.get())` was never called, or it was called after `startDevTools()` (in which case DevTools cached the absence). Set the engine first, then start DevTools.

### 16.7 `print` output doesn't show in DevTools Console

Default print writes to `stdout`. Install a print handler that forwards to `KUI_LOG_INFO`:

```cpp
engine->setPrintHandler([](const std::string& msg) {
    KUI_LOG_INFO("Lua", msg);
});
```

### 16.8 "Cannot upgrade element" for a custom element

`defineCustomElement` must run before the HTML containing the element is parsed, or before the element is actually inserted. Define custom elements at the top of your script, or in a separate script tag in `<head>`.

### 16.9 Memory grows with each event dispatch

A reference cycle - your Lua callback captures the element, and the element holds the listener. KUI breaks the cycle via `weak_ptr` internally, but if your callback also captures other Lua tables that hold the element, you can leak. Use `el:removeEventListener(type, id)` explicitly when removing nodes that won't be garbage-collected.

---

## 17. FAQ

### Q: Can I use Lua libraries from LuaRocks?

**A:** Yes, but the `package` module is disabled by default for sandbox safety. Re-enable it from C++ if you trust the source: `engine->lua().open_libraries(sol::lib::package)`. Then `require("...")` works, with `package.path` honoured.

### Q: Can I mix Lua and JavaScript in the same KUI document?

**A:** No. KUI v3.1.0 ships only the Lua backend. The architecture supports swappable engines (see `ScriptEngine` in `script_engine.h`), but no JS engine is available out of the box.

### Q: Why can't I call `os.exit()` from a script?

**A:** The `os` module is sandboxed out. Use the host-side exit pattern: from C++, watch for a sentinel event from Lua (`document:dispatchEvent("app-exit")`) and call your own exit logic.

### Q: 使用哪个 Lua 版本？是否保证 LuaJIT 兼容？

**A:** 版本由构建时找到的 Lua 依赖决定，本文不固定为 5.4。用实际引擎中的 `_VERSION` 确认运行时主/次版本，再核对 SDK 构建记录中的补丁版本。不要仅凭本机 `lua` 命令或另一个依赖目录的头文件判断 DLL 内使用的版本；也不要从使用 sol2 推导出 LuaJIT 的兼容保证。

### Q: How fast is a `document:getElementById` call from Lua?

**A:** About 90-110 ns per call on a modern CPU. The actual hash lookup is ~30 ns; the rest is sol2 marshalling. For tight loops, cache the result in a local.

### Q: Can I do `el:foo()` syntax for any C++ method?

**A:** Only for methods explicitly registered in the `lua_*_bindings.cpp` files. Adding a new method is a one-line sol2 binding in the appropriate registration function.

### Q: Does Lua have access to `HttpClient`?

**A:** 当前 Lua 绑定没有直接暴露 `HttpClient`。由 C++ 宿主发起 `HttpClient::fetchAsync` 后，工作线程回调只传递普通响应数据；通过宿主的 UI 投递机制/队列，在 UI 线程确认引擎和文档仍存活后再调用 Lua。禁止从 HTTP 回调直接 `eval` / `callFunction`。线程和销毁处理见 [3.2 Threading model](#32-threading-model)。

### Q: Can I save Lua tables to disk?

**A:** Not via `io` (sandboxed). Use the JSON binding: `JSON.stringify(myTable)` -> pass to C++ -> write to disk. Or open the `io` module if you trust your scripts.

### Q: Does Lua have access to the 3D engine?

**A:** A subset. `examples/native/script/04_lua_animation/` shows shader and canvas integration. Full Filament binding is intentionally minimal; for heavy 3D work, drive the renderer from C++ and let Lua control parameters.

### Q: Will my Lua scripts continue to work after a KUI minor upgrade?

**A:** Yes within the v3.x series. The Lua binding surface is part of the same SameMajorVersion contract as the C++ API. Method names, event names, and global identifiers are stable.

---

## 18. See Also

- `tutorial.md` - General KUI walkthrough
- `api_reference.md` - Full SDK API index (search for `ScriptEngine`)
- [`devtools.md`](./devtools.md) - DevTools server (the debugger backend for Lua)
- `architecture.md` - KUI engine internals
- [`features.md`](./features.md) - HTML & CSS feature matrix (what's available to manipulate from Lua)
- `faq.md` - Cross-cutting FAQs
- `../examples/native/script/` - Five runnable Lua + DOM example programs
- `../examples/tank_game_lua/` - Complete game driven from Lua
- `<kui/script/script_engine.h>` - Public C++ header
- [sol2 documentation](https://sol2.readthedocs.io/) - The C++ <-> Lua binding library under the hood
- [Lua reference manuals](https://www.lua.org/manual/) - 选择与实际引擎 `_VERSION` 一致的版本

