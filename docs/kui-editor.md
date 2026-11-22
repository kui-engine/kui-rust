# `<kui-editor>` - Code Editor Component

> **Audience:** Developers embedding the KUI code editor in their applications. This document is the authoritative reference for `<kui-editor>`'s HTML attributes, default keybindings, the 200+ Lua methods it exposes, and every higher-level feature (multi-cursor, code folding, tree-sitter highlighting, PCRE2 regex find/replace, LSP integration, Git diff gutter, snippets, ghost text, document symbols, fuzzy picker).
>
> **Outcome:** After reading this document you will know how to drop the editor into any HTML page, configure it for your language, drive it from Lua, and integrate it with your application's command system, language server, version control, and AI suggestion backend.

**Applies to:** `kui_ext_editor` v1.9.0 (M1-06, 2026-05-13). Requires KUI Host API **v9** or later.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Quick Start](#2-quick-start)
3. [Architecture](#3-architecture)
4. [HTML Attributes](#4-html-attributes)
5. [Themes](#5-themes)
6. [Languages & Syntax Highlighting](#6-languages--syntax-highlighting)
7. [Default Keybindings](#7-default-keybindings)
8. [Lua API: Buffer & Caret](#8-lua-api-buffer--caret)
9. [Lua API: Selection](#9-lua-api-selection)
10. [Lua API: Clipboard & Undo](#10-lua-api-clipboard--undo)
11. [Lua API: Movement & Deletion](#11-lua-api-movement--deletion)
12. [Lua API: Theme, Font, Language](#12-lua-api-theme-font-language)
13. [Find / Replace / Regex](#13-find--replace--regex)
14. [Multi-Cursor & Column Selection](#14-multi-cursor--column-selection)
15. [Code Folding](#15-code-folding)
16. [Find / Replace Widget](#16-find--replace-widget)
17. [Minimap](#17-minimap)
18. [LSP Integration](#18-lsp-integration)
19. [Git Diff Gutter & Blame](#19-git-diff-gutter--blame)
20. [Sticky Scroll, Brackets & Indent Guides](#20-sticky-scroll-brackets--indent-guides)
21. [Breadcrumb](#21-breadcrumb)
22. [Snippets](#22-snippets)
23. [Ghost Text (Inline Suggestions)](#23-ghost-text-inline-suggestions)
24. [Line Transforms & Navigation](#24-line-transforms--navigation)
25. [Document Symbols](#25-document-symbols)
26. [Fuzzy Picker](#26-fuzzy-picker)
27. [Built-in Command Registry](#27-built-in-command-registry)
28. [Events & Versioning](#28-events--versioning)
29. [Performance Notes](#29-performance-notes)
30. [Complete Examples](#30-complete-examples)
31. [Troubleshooting](#31-troubleshooting)
32. [FAQ](#32-faq)
33. [See Also](#33-see-also)

---

## 1. Overview

`<kui-editor>` is a full-featured code editor shipped as a DLL extension (`kui_ext_editor.dll`) for KUI. It implements roughly the same surface area as VS Code's `monaco-editor` but built natively in C++ on top of the KUI extension API.

### 1.1 What it provides

- **Full editing model**: per-line buffer, UTF-8 aware, with bounded 1000-step undo / redo and 500 ms typing-merge windows.
- **Multi-cursor + column/block selection**: standard VS Code shortcuts (`Ctrl+D`, `Ctrl+Alt+Up/Down`, `Shift+Alt+Drag`, etc.).
- **Tree-sitter syntax highlighting** for 7 languages with vendored `highlight.scm` / `folds.scm` queries.
- **PCRE2 regex** find / replace with 7 option flags, including in-selection search and preserve-case replace.
- **Code folding** with auto-detected fold ranges, collapse/expand-all.
- **Find/Replace widget overlay** with Tab to switch fields, Enter to navigate matches.
- **Minimap** sidebar (toggleable).
- **LSP client**: spawn language server, didOpen/didChange/didClose auto-sync, 8 request types (hover / completion / definition / references / signature-help / formatting / rename / prepare-rename), full diagnostics with squiggle paint + gutter markers.
- **Git diff gutter** + **inline blame** + **sticky scroll** + **breadcrumb**.
- **Bracket-pair colorization** + **indent guides** + **occurrence highlight** + **bracket match**.
- **Snippets** with LSP-style `${1:placeholder}` + `${1|choice1,choice2|}` + `${VAR}` variable expansion.
- **Ghost text** (inline AI suggestions): set/clear/accept/accept-word.
- **Document symbols** + goto-line / goto-symbol UI surface.
- **Fuzzy picker** (foundation for Quick Open / Command Palette / Goto Symbol).
- **3 built-in themes** (Catppuccin Mocha, Latte, High-Contrast) + tokenizer color mapping.

### 1.2 Distribution

The editor ships as a single DLL: `kui_ext_editor.dll` (~8.9 MB on Windows x64). The bundle contains:

- Tree-sitter runtime (`libtree-sitter`) + 7 vendored grammars (~5 MB combined)
- PCRE2 regex engine (~600 KB)
- KUI host glue + editor implementation (~2 MB)

Drop it into your host's `extensions/` directory and the host will load it automatically on next launch.

### 1.3 Capabilities matrix

| Capability | Status | Section |
|------------|--------|---------|
| UTF-8 editing | Yes | [§8](#8-lua-api-buffer--caret) |
| Multi-cursor | Yes | [§14](#14-multi-cursor--column-selection) |
| Column / block selection | Yes | [§14](#14-multi-cursor--column-selection) |
| Tree-sitter highlight (7 languages) | Yes | [§6](#6-languages--syntax-highlighting) |
| PCRE2 regex find / replace | Yes | [§13](#13-find--replace--regex) |
| Code folding | Yes | [§15](#15-code-folding) |
| Minimap | Yes | [§17](#17-minimap) |
| LSP client | Yes | [§18](#18-lsp-integration) |
| Git diff gutter | Yes | [§19](#19-git-diff-gutter--blame) |
| Snippets | Yes | [§22](#22-snippets) |
| Ghost text | Yes | [§23](#23-ghost-text-inline-suggestions) |
| Document symbols / picker | Yes | [§25](#25-document-symbols) |
| Soft word wrap | No | - |
| Per-line gutter breakpoints (paintable) | No | - |
| Inline images | No | - |
| Per-document language modes | One language per element | - |

---

## 2. Quick Start

### 2.1 Two-line embed

```html
<kui-editor language="cpp" theme="mocha" id="ed"></kui-editor>

<script type="text/lua">
    local ed = document:querySelector("#ed")
    ed:setValue([[
#include <iostream>
int main() {
    std::cout << "Hello, KUI!" << std::endl;
    return 0;
}
]])
end
</script>
```

That's it. You now have a fully-featured editor with syntax highlighting, multi-cursor, find/replace, undo/redo, clipboard, IME support, and 200+ Lua methods bound to the element.

### 2.2 Pre-requisites

1. Host built against KUI SDK >= v3.1.0 (API version 9).
2. `kui_ext_editor.dll` placed in your host's extensions directory (typically `<exe>/extensions/`).
3. Optional: `tree_sitter_*.dll` co-located if your build dynamically loads grammars (the bundled DLL statically links all 7).

### 2.3 Sizing

The editor reports its natural size as `(availableWidth, min(allLinesH, 400px))`. To pin a different height, just use CSS:

```html
<kui-editor language="lua" style="height: 600px; width: 100%;"></kui-editor>
```

`width` and `height` work like any other CSS-sized box.

---

## 3. Architecture

The editor was built in well-defined milestones, and the source files are tagged with the milestone marker they belong to. Reading the milestone list is the fastest way to understand which subsystem owns which feature.

### 3.1 Milestone map

| Milestone | Files | Subsystem |
|-----------|-------|-----------|
| **M0-S1..S4** | `editor_state.{h,cpp}`, `editor_plugin.cpp`, `editor_paint.cpp`, `editor_events.cpp` | Buffer, caret, IME, blink, viewport, gutter, theme, font |
| **M1-04..06** | `editor_search.cpp` | PCRE2 regex find / replace |
| **M1-07** | `editor_folding.cpp` | Code folding |
| **M1-08** | `editor_state.cpp` (auto-pair / smart-indent) | Typing affordances |
| **M1-09** | `editor_findwidget.cpp` | Find/Replace widget overlay |
| **M1-10** | `editor_paint.cpp` (minimap section) | Minimap |
| **M1-11** | `editor_highlight.cpp` | Occurrence + bracket match |
| **M1-12** | `editor_lineops.cpp` | Toggle comment, move/copy/delete/sort line |
| **M1-CARET** | `editor_cursors.cpp` | Multi-cursor (`cursors[]`, compound undo) |
| **M1-TS-A..E** | `editor_treesitter.{h,cpp}`, `editor_treesitter_queries.cpp` | Tree-sitter parser, highlight.scm, folds.scm, AST select |
| **M2-14** | `editor_lsp_*.{h,cpp}`, `editor_lua_api_lsp.cpp` | LSP client + 8 request types + diagnostics |
| **M3-01** | `editor_git.cpp`, `editor_lua_api_git.cpp` | Git diff gutter |
| **M3-02** | `editor_blame.cpp`, `editor_lua_api_blame.cpp` | Inline blame |
| **M3-03** | `editor_lua_api_sticky.cpp` | Sticky scroll |
| **M3-04** | `editor_brackets.cpp`, `editor_lua_api_brackets.cpp` | Bracket colorization + indent guides |
| **M3-05** | `editor_lua_api_breadcrumb.cpp` | Breadcrumb |
| **M3-06** | `editor_snippet.cpp`, `editor_lua_api_snippet.cpp` | Snippets |
| **M4-02** | `editor_column.cpp`, `editor_lua_api_column.cpp` | Column / block selection |
| **M4-03** | `editor_ghost.cpp`, `editor_lua_api_ghost.cpp` | Ghost text (AI suggestions) |
| **M4-04** | `editor_lua_api_transform.cpp`, `editor_lua_api_navigate.cpp`, `editor_symbols.cpp` | Line transforms, navigation, document symbols |
| **M4-05** | `editor_picker.cpp`, `editor_lua_api_picker.cpp` | Fuzzy picker |
| **M4-05b** | `editor_lua_api_commands.cpp` | Built-in command registry |
| **M4-06** | `editor_snippet_var.cpp` | Snippet variable context |
| **M5-04/05** | `editor_paint.cpp` (partial-rect repaint) | Performance instrumentation |

### 3.2 Per-element state

Each `<kui-editor>` instance gets a heap-allocated `EditorState` struct registered in `g_states[void*]` keyed by the host's element pointer. Lookups are O(1) via `unordered_map`. The struct holds everything mutable:

- `lines: vector<string>` - buffer (will move to Piece Tree post-M1)
- `cursors: vector<Cursor>` - multi-cursor array (>= 1)
- `caretLine/Col/anchorLine/Col/goalCol` - mirror of `cursors[0]` for legacy callers
- `undoStack`, `redoStack` - bounded at 1000 entries, 500 ms typing merge window
- `composingText`, `composingCaretPos` - IME state
- `findMatches`, `findCurrentIdx` - search engine results
- `foldedStartLines` - active folds
- `tsHighlighter` (opaque) - tree-sitter parser + tree
- `lspManager` (opaque) - LSP client + state machine
- `theme` - color palette (default `mocha`)
- ... and dozens more.

Full field list: see `editor_state.h` (~755 lines, well-commented).

### 3.3 Threading

All editor callbacks run on the **KUI main thread**. The LSP client owns a worker thread for stdin/stdout I/O; results are buffered and consumed during `onTick`. No state mutex needed.

---

## 4. HTML Attributes

The editor observes 6 attributes. Their **initial** values (declared in markup) are applied when the element connects (`onConnected`, per the standard `connectedCallback` contract — KUI-055); setting any of them **after** connection fires `onAttributeChanged` and updates state synchronously. Either path produces the same result, so `<kui-editor theme="latte">` renders its first frame in `latte` without any post-connect `setAttribute`.

| Attribute | Type | Default | Effect |
|-----------|------|---------|--------|
| `value` | string | `""` | Replaces the entire buffer. Set once before the element is parsed for initial content, or via `setAttribute` to reset programmatically. Each set produces an undo entry. |
| `language` / `data-language` | string | `""` (no highlight) | Tree-sitter grammar selection. Case-insensitive. See [§6](#6-languages--syntax-highlighting) for accepted names. Unknown names disable tree-sitter and fall back to the lightweight tokenizer. |
| `theme` / `data-theme` | string | `mocha` | Color palette. Accepted: `mocha`, `latte`, `hc` (case-insensitive). Unknown -> `mocha`. |
| `font-size` | px | `14` | Range `4..96`. Accepts `"14"`, `"14px"`, `"14.5"` (decimal). `lineHeight` is auto-recalculated as `fontSize * 1.4`. |
| `font-family` | string | `"Consolas, 'Courier New', monospace"` | CSS font-family string. Resolved via the host's CSS layer. |
| `tabindex` | int | auto-set to `"0"` | Required for the element to receive Tab focus. The editor automatically sets this in `onConnected` if absent. |
| `style` | string | `overflow:clip;cursor:text;` appended | `onConnected` injects `overflow:clip` (KUI-028 paint clipping) and `cursor:text` (KUI-031 I-beam over the editing surface) into the inline style **only when the author has not declared `overflow`/`cursor`** — declare your own value to opt out. When the editor owns the cursor (author did not declare one), it auto-switches it to `default` (arrow) while the pointer is over a self-drawn scrollbar region — minimap, classic vertical bar, or the KUI-053 horizontal bar — and back to `text` over the text area (KUI-056). An author-declared `cursor` is never overridden. |

### 4.1 Setting attributes from HTML

```html
<kui-editor
    language="cpp"
    theme="mocha"
    font-size="13"
    font-family="JetBrains Mono, Consolas, monospace"
    value="int main() { return 0; }">
</kui-editor>
```

These author-declared values take effect at connection time — the editor's first painted frame already reflects them. There is no need to re-issue `setAttribute` after `loadHTML` to "wake up" the declared theme / font-size / language (this was required before KUI-055, when `onConnected` only read `value`).

### 4.2 Setting attributes from Lua

```lua
local ed = document:querySelector("kui-editor")
ed:setAttribute("language", "python")
ed:setAttribute("theme", "latte")
ed:setAttribute("font-size", "16")
```

Or use the dedicated Lua methods which take effect without an attribute round-trip:

```lua
ed:setLanguage("python")
ed:setTheme("latte")
ed:setFontSize(16)
ed:setFontFamily("JetBrains Mono")
```

### 4.3 Attributes the editor does NOT observe

The host's standard attributes (`id`, `class`, `style`, `tabindex` after the initial set, `data-*` other than the four above) are stored on the element but do not trigger editor-specific behavior. CSS can style the host element's box (margin, border, background of the gutter strip, etc.) but cannot override the editor's internal paint colors - use a theme for that.

---

## 5. Themes

The editor ships with **3 built-in themes**. Each is a fully-populated `EditorTheme` struct (~30 color slots covering background, gutter, current-line highlight, selection, all token categories, diagnostic squiggles, git diff, sticky scroll, bracket pair colorization).

### 5.1 The three presets

| Theme name | Aliases | Description | Use case |
|------------|---------|-------------|----------|
| `mocha` | `dark` | Catppuccin Mocha. Soft pastel dark. Default. | Long-session editing in low-light environments |
| `latte` | `light` | Catppuccin Latte. Warm pastel light. | Daylight / projector / paper-like reading |
| `hc` | `high-contrast` | WCAG AA-compliant high-contrast dark. | Accessibility, screenshots, video recording |

Name lookup is case-insensitive (`MOCHA`, `Mocha`, `mocha` all work). Unknown names fall back to `mocha`.

### 5.2 Setting a theme

```lua
ed:setTheme("latte")          -- light theme
ed:setTheme("hc")             -- high contrast
print(ed:getTheme())          -- "hc"
```

Or via attribute:

```html
<kui-editor theme="latte">...</kui-editor>
```

### 5.3 Color slots

Each theme populates the following slots (defined in `editor_types.h::EditorTheme`):

| Slot | Used by |
|------|---------|
| `background`, `foreground` | Text area |
| `gutterBackground`, `gutterForeground`, `gutterCurrentLine` | Line number gutter |
| `currentLineBackground` | Active line highlight |
| `selectionBackground` | Selection box |
| `caretColor` | Caret bar |
| `keywordColor`, `stringColor`, `numberColor`, `commentColor`, `functionColor`, `typeColor`, `variableColor`, `operatorColor`, `punctuationColor`, `attributeColor`, `propertyColor`, `escapeColor` | Token categories (mapped from tree-sitter capture names) |
| `errorSquiggle`, `warningSquiggle`, `infoSquiggle`, `hintSquiggle` | LSP diagnostic underlines |
| `gitAddedGutter`, `gitModifiedGutter`, `gitDeletedGutter` | Git diff gutter strip |
| `stickyBackground`, `stickyBorder` | Sticky scroll overlay |
| `bracketColors[6]` | Bracket-pair colorization ladder |
| `indentGuideColor`, `indentGuideActiveColor` | Indent guides |

To author a custom theme, you currently must rebuild the DLL with a new `themeXxx()` function added to `editor_theme.cpp`. A `setCustomTheme(jsonString)` Lua method is on the roadmap (M5+).

---

## 6. Languages & Syntax Highlighting

The editor uses **tree-sitter** as its syntax engine. Grammars are vendored and statically linked into the DLL - no separate `.dll` / `.so` files to ship.

### 6.1 Supported languages

| Canonical name | Aliases | Grammar | Highlight + folds |
|----------------|---------|---------|-------------------|
| `c` | - | tree_sitter_c | Yes / Yes |
| `cpp` | `c++` | tree_sitter_cpp | Yes / Yes |
| `javascript` | `js` | tree_sitter_javascript | Yes / Yes |
| `typescript` | `ts` | tree_sitter_typescript | Yes / Yes |
| `tsx` | - | tree_sitter_tsx | Yes (combined JS+TS scm) / Yes |
| `python` | `py` | tree_sitter_python | Yes / Yes |
| `lua` | - | tree_sitter_lua | Yes / Yes |

### 6.2 Selecting a language

```lua
ed:setLanguage("python")
print(ed:getLanguage())   -- "python"

ed:setLanguage("")        -- disables tree-sitter, falls back to light tokenizer
ed:setLanguage(nil)       -- same as setLanguage("")
```

Or via attribute:

```html
<kui-editor language="cpp">...</kui-editor>
```

### 6.3 How highlighting works

1. On `setLanguage(name)`, the editor:
   - Loads the matching grammar via `lookupTsLanguage(name)`.
   - Loads the vendored `highlight.scm` (queries the AST for capture names).
   - Loads the vendored `folds.scm` (queries for foldable ranges).
   - Pre-resolves capture names to internal `TokenKind` buckets at compile time (zero string compares in the paint hot path).

2. On every buffer mutation, the editor:
   - Calls `ts_parser_parse_string` (incremental - reuses the previous tree).
   - Runs the highlight query against the new tree.
   - Caches per-line `TokenSpan` arrays keyed by `cacheVersion`.

3. On every paint, the editor:
   - Reads cached spans for the visible lines.
   - Maps each span's `TokenKind` to a `theme.xxxColor` slot.
   - Issues one `dlDrawTextStyled` call per styled run.

For buffers > 2 MB the tree-sitter parse falls back to "no highlight" mode to keep paint under 16 ms (constant `kTsMaxBufferBytes` in `editor_treesitter.h`). The light fallback tokenizer is plain regex-style C/JS/Python keyword highlight.

### 6.4 Querying the AST

For debugging / tooling, the editor exposes the live AST:

```lua
print(ed:getAstSummary())       -- top-N nodes, default = 80
print(ed:getAstSummary(200))    -- top-200 nodes
print(ed:getTsDebugInfo())      -- grammar / query state info

-- Per-line token spans (offset / length / kind tuples):
local spans = ed:getLineTokens(0)   -- comma-separated triples
```

### 6.5 AST-based smart selection (M1-TS-E)

```lua
ed:selectionGrow()      -- expand selection to enclosing AST node
ed:selectionShrink()    -- shrink back toward the original selection
```

Works regardless of buffer content; finds the smallest TSNode containing the current caret and expands by one AST level each call.

---

## 7. Default Keybindings

All shortcuts are processed in `editor_events.cpp::onKeyEvent`. Returning `1` from the callback consumes the event so the host doesn't double-handle it.

### 7.1 Editing

| Shortcut | Action |
|----------|--------|
| `Printable Unicode` (no Ctrl/Alt) | Insert character at caret (respects auto-pair if enabled) |
| `Enter` | Insert newline; smart-indent copies leading whitespace; auto-expand between matching `{}`/`[]`/`()` |
| `Tab` | Insert 4 spaces (tagged `EditReason::Tab` so it never merges with regular typing in undo) |
| `Backspace` | Delete char before caret, or selection if any |
| `Delete` | Delete char after caret, or selection if any |
| `Ctrl+Backspace` | Delete word before caret |
| `Ctrl+Delete` | Delete word after caret |

### 7.2 Movement

| Shortcut | Action |
|----------|--------|
| `Arrow Left/Right/Up/Down` | Move caret one char/line. `Shift` extends selection. |
| `Home` / `End` | Move to line start / end. `Shift` extends. |
| `Ctrl+Arrow Left/Right` | Move by word boundary. `Shift` extends. |
| `PageUp` / `PageDown` | Scroll one viewport. `Shift` extends. |

### 7.3 Clipboard

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Copy selection; if no selection, copy current line + `\n` (VS Code behavior) |
| `Ctrl+X` | Cut selection; if no selection, cut current line + `\n` |
| `Ctrl+V` | Paste (line-aware: multi-cursor paste splits clipboard by `\n` and distributes one line per cursor when cursor count matches line count) |

### 7.4 Undo

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo (up to 1000 steps; same-reason ops within 500 ms merge into one) |
| `Ctrl+Y` | Redo |
| `Ctrl+Shift+Z` | Redo (mac/vim alternative) |

### 7.5 Multi-cursor

| Shortcut | Action |
|----------|--------|
| `Ctrl+D` | Add cursor at next occurrence of current selection (or current word) |
| `Ctrl+Shift+L` | Add cursors at all occurrences of selection |
| `Ctrl+Alt+ArrowUp` | Add cursor on the line above |
| `Ctrl+Alt+ArrowDown` | Add cursor on the line below |
| `Escape` | Remove all secondary cursors (only primary remains) |

### 7.6 Column / block selection

| Shortcut | Action |
|----------|--------|
| `Shift+Alt+Drag (mouse)` | Start a column / block selection rectangle |
| `Ctrl+Shift+Alt+ArrowLeft/Right/Up/Down` | Extend column rectangle by one cell |

### 7.7 Line operations

| Shortcut | Action |
|----------|--------|
| `Ctrl+/` | Toggle line comment (language-aware: `//`, `#`, `--`, `<!-- -->`) |
| `Alt+ArrowUp/ArrowDown` | Move current line (or selection block) up / down |
| `Shift+Alt+ArrowUp/ArrowDown` | Copy current line up / down |
| `Ctrl+Shift+K` | Delete current line (or selection's lines) |

### 7.8 Ghost text (when active)

| Shortcut | Action |
|----------|--------|
| `Tab` | Accept entire ghost text |
| `Esc` | Dismiss ghost text |
| Any caret movement | Dismiss ghost text silently |

### 7.9 Find widget (when open)

| Shortcut | Action |
|----------|--------|
| `Enter` | Submit next match |
| `Shift+Enter` | Submit previous match |
| `Tab` | Swap active field (find <-> replace) |
| `Backspace` | Delete in active field |
| `Escape` | Close widget |
| Printable char | Append to active field |

The widget is opened from your application code: `ed:openFindWidget(0)` (find mode) or `ed:openFindWidget(1)` (replace mode). Bind these to `Ctrl+F` / `Ctrl+H` in your host's input layer.

---

## 8. Lua API: Buffer & Caret

All methods are called via Lua's `:` syntax: `ed:setValue("...")`. The Lua side uses the host's typed-value channel; no `lua54.dll` dependency is required.

### 8.1 Buffer

| Method | Signature | Description |
|--------|-----------|-------------|
| `setValue(s)` | `(string) -> void` | Replace entire buffer. Records one undo entry. |
| `getValue()` | `() -> string` | Return entire buffer as one string with `\n` separators. |
| `getLineCount()` | `() -> int` | Number of lines (always >= 1). |
| `insertText(s)` | `(string) -> void` | Insert `s` at caret (replaces selection if any). Multi-cursor aware. |

```lua
ed:setValue("line1\nline2\nline3")
print(ed:getLineCount())     -- 3
print(ed:getValue())         -- "line1\nline2\nline3"
ed:insertText("HELLO\n")
```

### 8.2 Caret

| Method | Signature | Description |
|--------|-----------|-------------|
| `getCaretLine()` | `() -> int` | Caret line (0-indexed). |
| `getCaretCol()` | `() -> int` | Caret column in **bytes** within the line (UTF-8). |
| `setCaret(line, col)` | `(int, int) -> void` | Move caret. Clamps to buffer bounds. Collapses any active selection. |
| `getVersion()` | `() -> int` | Monotonic counter; bumps on every state change (buffer / caret / scroll / focus / blink / IME). |
| `getBufferContentVersion()` | `() -> int` | Bumps **only** on buffer mutations (typing / paste / undo / setValue). Use for change detection that doesn't react to mere caret moves. |
| `focus()` | `() -> void` | Programmatically focus the editor (host `requestFocus`). |

```lua
ed:setCaret(0, 0)        -- jump to start
print(ed:getCaretLine(), ed:getCaretCol())   -- 0  0

-- Poll for changes:
local lastV = ed:getBufferContentVersion()
-- ... later ...
local nowV = ed:getBufferContentVersion()
if nowV ~= lastV then onBufferChanged() end
```

> **Note on column units:** All caret columns are **byte offsets** in the UTF-8 line. ASCII chars take 1 byte, ASCII letters with accents 2 bytes, CJK 3 bytes, emoji 4 bytes. The editor enforces caret-at-char-boundary invariants - `setCaret(0, 1)` mid-2-byte-char will snap to the nearest boundary.

---

## 9. Lua API: Selection

The selection model uses **anchor + caret**: the selection runs from `(anchorLine, anchorCol)` to `(caretLine, caretCol)`. When anchor == caret, there is no selection (the editor is in "collapsed caret" mode).

### 9.1 Query

| Method | Signature | Description |
|--------|-----------|-------------|
| `getAnchorLine()` | `() -> int` | Selection anchor line. |
| `getAnchorCol()` | `() -> int` | Selection anchor byte column. |
| `hasSelection()` | `() -> int` | `1` if any non-empty selection across any cursor; `0` otherwise. |
| `getSelectedText()` | `() -> string` | Concatenation of all cursors' selected text. |

### 9.2 Mutate

| Method | Signature | Description |
|--------|-----------|-------------|
| `setSelection(aL, aC, cL, cC)` | `(int, int, int, int) -> void` | Set both anchor and caret. |
| `selectAll()` | `() -> void` | Anchor at `(0,0)`, caret at end of buffer. |
| `deleteSelection()` | `() -> int` | Delete selection; returns `1` if anything was deleted. Multi-cursor aware. |
| `selectWord(line, col)` | `(int, int) -> void` | Expand to enclosing word at the given byte position. Used by mouse double-click. |
| `selectLine(line)` | `(int) -> void` | Select the entire line. Used by mouse triple-click. |
| `selectionGrow()` | `() -> void` | AST-aware: expand to enclosing tree-sitter node. (M1-TS-E) |
| `selectionShrink()` | `() -> void` | Inverse of grow. |

```lua
ed:setSelection(0, 0, 0, 5)        -- select first 5 bytes of line 0
print(ed:hasSelection())           -- 1
print(ed:getSelectedText())        -- e.g. "hello"
ed:deleteSelection()               -- delete it

ed:selectAll()
ed:deleteSelection()               -- empty the buffer
```

---

## 10. Lua API: Clipboard & Undo

### 10.1 Clipboard

All clipboard methods route through the host's `getClipboardText` / `setClipboardText` (Host API v9). UTF-8 text only.

| Method | Signature | Description |
|--------|-----------|-------------|
| `copy()` | `() -> int` | Copy selection to system clipboard; if no selection, copy current line + `\n`. Returns `1` if anything was copied. |
| `cut()` | `() -> int` | Same as copy + delete. |
| `paste()` | `() -> int` | Read system clipboard and insert at caret. Multi-cursor + line-aware (see [§14](#14-multi-cursor--column-selection)). Returns `1` if anything was pasted. |

```lua
ed:selectAll()
ed:copy()         -- whole buffer to clipboard
ed:setValue("")
ed:paste()        -- paste it back
```

### 10.2 Undo / Redo

| Method | Signature | Description |
|--------|-----------|-------------|
| `undo()` | `() -> int` | Undo one step (or one compound batch). Returns `1` on success. |
| `redo()` | `() -> int` | Redo one step. Returns `1` on success. |
| `canUndo()` | `() -> int` | `1` if the undo stack is non-empty. |
| `canRedo()` | `() -> int` | `1` if the redo stack is non-empty. |

```lua
ed:setValue("hello")
ed:insertText(" world")
print(ed:getValue())     -- "hello world"
ed:undo()
print(ed:getValue())     -- "hello"
ed:redo()
print(ed:getValue())     -- "hello world"
```

**Merge semantics:** Same-reason ops within 500 ms merge into one undo entry. Newlines always break the merge. Reasons: `TypingChar` (regular typing), `Backspace`, `DeleteForward`, `Tab` (4-space tab), `Paste`, `Cut`, `Replace`, `LineMove`, etc. So `abc<Backspace>def` produces 2 undo entries (typing + backspace + typing), and `Ctrl+Z` walks back through them one batch at a time.

**Multi-cursor batching:** When all secondary cursors produce one edit each (e.g. typing `Ctrl+D`-selected occurrences), the N `EditOp`s share one `batchId` and collapse to a single `Ctrl+Z` atom.

---

## 11. Lua API: Movement & Deletion

All movement methods accept an optional `extend` argument: pass `1` to extend the selection (mirrors Shift modifier), `0` or omit to collapse.

### 11.1 Movement

| Method | Signature | Description |
|--------|-----------|-------------|
| `moveLeft([extend])` | `(int?) -> void` | One char left. |
| `moveRight([extend])` | `(int?) -> void` | One char right. |
| `moveUp([extend])` | `(int?) -> void` | One line up; preserves `goalCol` across short lines (VS Code behavior). |
| `moveDown([extend])` | `(int?) -> void` | One line down. |
| `moveHome([extend])` | `(int?) -> void` | Line start. |
| `moveEnd([extend])` | `(int?) -> void` | Line end. |
| `movePageUp([extend])` | `(int?) -> void` | Scroll one viewport up. |
| `movePageDown([extend])` | `(int?) -> void` | Scroll one viewport down. |
| `moveWordLeft([extend])` | `(int?) -> void` | One word left. |
| `moveWordRight([extend])` | `(int?) -> void` | One word right. |

All ten methods are multi-cursor aware: each cursor moves independently, and overlapping cursors collapse via `mergeAndSort`.

```lua
ed:setCaret(5, 10)
ed:moveWordLeft(1)        -- extend selection to previous word
print(ed:getSelectedText())

ed:moveDown(0)            -- collapse to single caret one line below
```

### 11.2 Deletion

Each method returns `1` if the buffer was modified, `0` otherwise.

| Method | Signature | Description |
|--------|-----------|-------------|
| `backspace()` | `() -> int` | Delete char before caret (or selection). |
| `deleteForward()` | `() -> int` | Delete char after caret. |
| `deleteWordLeft()` | `() -> int` | Delete word before caret. |
| `deleteWordRight()` | `() -> int` | Delete word after caret. |

```lua
ed:setValue("hello world")
ed:setCaret(0, 11)
ed:deleteWordLeft()        -- buffer is "hello "
ed:deleteWordLeft()        -- buffer is ""
```

### 11.3 Viewport

| Method | Signature | Description |
|--------|-----------|-------------|
| `setVisibleLineCount(n)` | `(int) -> void` | Override the auto-computed page step for `movePageUp/Down`. Useful when the editor sits in a custom-sized container. |

---

## 12. Lua API: Theme, Font, Language

| Method | Signature | Description |
|--------|-----------|-------------|
| `setTheme(name)` | `(string) -> void` | `mocha` / `latte` / `hc` (case-insensitive). |
| `getTheme()` | `() -> string` | Returns the last name passed to `setTheme` (or attribute). |
| `setFontSize(px)` | `(int) -> void` | Range `4..96`. Recomputes `lineHeight = px * 1.4`. |
| `getFontSize()` | `() -> int` | Current font size. |
| `setFontFamily(s)` | `(string) -> void` | CSS font-family string. |
| `getFontFamily()` | `() -> string` | Current font family. |
| `setLanguage([name])` | `(string?) -> void` | Tree-sitter grammar. Omit / empty / nil = disable highlighting. |
| `getLanguage()` | `() -> string` | Current language name (`""` if disabled). |
| `getAstSummary([maxNodes])` | `(int?) -> string` | Debug: top-N AST nodes as text. Default `maxNodes=80`. |
| `getLineTokens(line)` | `(int) -> string` | Per-line token spans as comma-separated `offset,length,kind` triples. |
| `getTsDebugInfo()` | `() -> string` | Grammar / query state debug string. |

```lua
ed:setLanguage("rust")        -- unknown -> highlighting disabled, returns ""
print(ed:getLanguage())       -- ""

ed:setLanguage("cpp")
print(ed:getLanguage())       -- "cpp"

ed:setTheme("latte")
ed:setFontSize(16)
ed:setFontFamily("JetBrains Mono, Consolas")
```

---

## 13. Find / Replace / Regex

The editor's search engine supports both plain-text and PCRE2 regex with 7 option flags. Matches are computed eagerly once on each `findSet` and cached until the next buffer mutation (tracked by `bufferContentVersion`).

### 13.1 Option flags (bitmask)

`findSet(pattern, flags)` takes a flags bitmask:

| Bit | Constant value | Flag | Effect |
|-----|---------------|------|--------|
| 0 | `1` | `caseSensitive` | When unset, match is case-insensitive |
| 1 | `2` | `wholeWord` | Match only on word boundaries |
| 2 | `4` | `inSelection` | Restrict matching to current selection range |
| 3 | `8` | `preserveCase` | On replace, preserve original casing pattern (`abc` -> `XYZ` becomes `xyz`, `ABC` -> `XYZ`, `Abc` -> `Xyz`) |
| 4 | `16` | `useRegex` | Treat pattern as PCRE2 regex |
| 5 | `32` | `multiline` | (Regex only) `^`/`$` match line boundaries |
| 6 | `64` | `dotAll` | (Regex only) `.` matches newlines |

Combine flags with bitwise OR (Lua: `+` works for non-overlapping bits, or use `bit32.bor` / `|`):

```lua
local PLAIN  = 0
local REGEX  = 16
local CI     = 0
local CS     = 1
local WW     = 2
ed:findSet("TODO", CS + WW)         -- whole-word, case-sensitive
ed:findSet("\\b[A-Z][a-z]+\\b", REGEX + CS)
```

### 13.2 Search methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `findSet(pattern, [flags])` | `(string, int?) -> int` | Recompute matches. Returns total match count. |
| `findClear()` | `() -> void` | Drop all matches. |
| `findCount()` | `() -> int` | Number of matches in last `findSet`. |
| `findCurrentIndex()` | `() -> int` | 0-based index of the "current" match (highlighted by paint). |
| `findNext()` | `() -> void` | Advance to next match; wraps. |
| `findPrev()` | `() -> void` | Step to previous match; wraps. |
| `findGetMatch(i)` | `(int) -> string` | Match `i` as `"sLine,sCol,eLine,eCol"`. |
| `findGetError()` | `() -> string` | PCRE2 compile error (empty if last pattern compiled cleanly). |

### 13.3 Replace methods

Each replacement is a separate undo entry (batched by `Ctrl+Z` if M1-05R is enabled in your build).

| Method | Signature | Description |
|--------|-----------|-------------|
| `findReplaceCurrent(repl)` | `(string) -> int` | Replace the current match. Supports `$1..$9` capture groups in regex mode. Returns `1` on success. |
| `findReplaceAll(repl)` | `(string) -> int` | Replace every match. Returns number of replacements made. |

```lua
ed:findSet("foo", 0)                  -- case-insensitive plain text
print(ed:findCount())                 -- e.g. 5
ed:findNext()                         -- highlight match 1
print(ed:findCurrentIndex())          -- 1

local n = ed:findReplaceAll("bar")
print("replaced", n, "occurrences")

-- Regex with capture groups
ed:findSet("(\\w+)\\s+(\\w+)", 16 + 1)     -- regex, case-sensitive
ed:findReplaceAll("$2 $1")                  -- swap first two words on each line
```

### 13.4 PCRE2 dialect

The engine is **PCRE2** 10.x with `PCRE2_UTF | PCRE2_UCP` enabled. Notable features:

- Unicode property classes: `\p{L}`, `\p{N}`, etc.
- Atomic groups: `(?>...)`
- Possessive quantifiers: `a++`, `a*+`
- Look-behind: `(?<=...)`, `(?<!...)`
- Named captures: `(?<name>...)` and `$<name>` in replacement
- `\K` (reset start)
- Verbs: `(*COMMIT)`, `(*FAIL)`, `(*ACCEPT)`

Things to be aware of:

- Patterns operate on **UTF-8 bytes**, but `\w` / `\d` / `\s` are Unicode-aware.
- `^` matches buffer start by default; pass flag `multiline=32` for per-line matching.
- `.` does not match `\n` by default; pass flag `dotAll=64` to enable.

If your pattern fails to compile, `findGetError()` returns the PCRE2 message:

```lua
ed:findSet("[a-z", 16)
print(ed:findGetError())    -- "missing terminating ] for character class"
```

---

## 14. Multi-Cursor & Column Selection

The editor stores cursors as an array `cursors[]` with `cursors[0]` being the **primary** (mirrored to the legacy `caretLine/Col/anchorLine/Col` fields). Secondary cursors live at indices `1..N`.

Cursors are kept sorted by selection start and merged on overlap (two cursors that touch collapse to one). The primary stays at index 0 across merges.

### 14.1 Multi-cursor methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `addCursor(line, col)` | `(int, int) -> void` | Add a secondary cursor (collapsed) at the given position. |
| `addCursorWithSelection(aL, aC, cL, cC)` | `(int,int,int,int) -> void` | Add a secondary cursor with a pre-existing selection. |
| `clearSecondaryCursors()` | `() -> void` | Reduce to primary only. Same as pressing `Escape`. |
| `getCursorCount()` | `() -> int` | Total cursor count (>= 1). |
| `getCursorAt(i)` | `(int) -> string` | Get `i`-th cursor as `"aL,aC,cL,cC,goalCol"`. |
| `addCursorAbove()` | `() -> void` | Bound to `Ctrl+Alt+ArrowUp`. |
| `addCursorBelow()` | `() -> void` | Bound to `Ctrl+Alt+ArrowDown`. |
| `selectNextOccurrence()` | `() -> void` | Bound to `Ctrl+D`. |
| `selectAllOccurrences()` | `() -> void` | Bound to `Ctrl+Shift+L`. |

### 14.2 Column / block selection methods

Column selection produces a rectangular region of N independent cursors (one per line in the column span). This is different from a single multi-line selection.

| Method | Signature | Description |
|--------|-----------|-------------|
| `columnSelectTo(aL, aC, cL, cC)` | `(int,int,int,int) -> void` | Set up a column rectangle from anchor `(aL,aC)` to caret `(cL,cC)`. Bound to `Shift+Alt+Drag` end events. |
| `cursorColumnSelectLeft/Right/Up/Down()` | `() -> void` | Extend the column rectangle by one cell. Bound to `Ctrl+Shift+Alt+Arrow`. |
| `hasColumnAnchor()` | `() -> int` | `1` if a column rectangle is being built. |
| `getColumnAnchorLine/Col()` | `() -> int` | Anchor coordinates. |
| `getColumnDragEndLine/Col()` | `() -> int` | Current end of the rectangle. |

```lua
-- Place cursors at column 4 of lines 0..2
ed:setCaret(0, 4)
ed:addCursor(1, 4)
ed:addCursor(2, 4)
ed:insertText("// ")       -- inserts at all 3 cursors simultaneously
```

### 14.3 Line-aware paste with multi-cursor

When the system clipboard contains exactly N lines and you have N cursors (N >= 2), paste distributes one line per cursor instead of duplicating the entire clipboard at every cursor. This matches VS Code's behavior:

- Clipboard `"foo\nbar\nbaz"` + 3 cursors -> cursor 0 gets `"foo"`, cursor 1 gets `"bar"`, cursor 2 gets `"baz"`.
- Clipboard `"foo"` + 3 cursors -> each cursor gets `"foo"` (broadcast).

### 14.4 Typing affordances

The editor implements auto-pair and smart-indent for common patterns. Both are on by default and toggleable per element:

| Method | Signature | Description |
|--------|-----------|-------------|
| `setAutoPairEnabled(b)` | `(int) -> void` | Toggle auto-pair. |
| `getAutoPairEnabled()` | `() -> int` | Query. |
| `setSmartIndentEnabled(b)` | `(int) -> void` | Toggle smart-indent. |
| `getSmartIndentEnabled()` | `() -> int` | Query. |

**Auto-pair:** Typing `(`, `[`, `{`, `"`, `'`, `` ` `` inserts the matching closer and parks the caret between them. Skips when the next char is already the closer (so `()` typed once produces `()` not `(())`).

**Smart-indent:** Enter copies leading whitespace from the current line. When the caret sits directly between a matching open/close pair (`{|}`), inserting Enter adds one extra indent level and a blank line between:

```cpp
if (x) {|}        // Enter pressed
// becomes
if (x) {
    |
}
```

---

## 15. Code Folding

Folding ranges come from the tree-sitter `folds.scm` query when a language is active, and from heuristic indent-based detection otherwise. Folded ranges are stored as a sorted set of start-line indices.

| Method | Signature | Description |
|--------|-----------|-------------|
| `getFoldRanges()` | `() -> string` | All fold ranges as `"sL,eL,sL,eL,..."`. |
| `toggleFold(line)` | `(int) -> void` | Collapse/expand the fold whose start is `line`. |
| `isLineHidden(line)` | `(int) -> int` | `1` if the line is inside a collapsed fold. |
| `collapseAllFolds()` | `() -> void` | Collapse every fold range. |
| `expandAllFolds()` | `() -> void` | Expand every fold range. |
| `setFoldingEnabled(b)` | `(int) -> void` | Master toggle. |
| `getFoldingEnabled()` | `() -> int` | Query. |
| `getFoldedCount()` | `() -> int` | Number of currently collapsed folds. |

```lua
local ranges = ed:getFoldRanges()      -- "0,10,3,7,..."
for sL in ranges:gmatch("(%-?%d+),%-?%d+") do
    ed:toggleFold(tonumber(sL))         -- collapse all
end
```

### 15.1 Folding behavior

- The start line of a fold remains visible (so a `▶` marker can render in the gutter).
- Lines in `(start, end]` are hidden via the paint layer; navigation skips them.
- `setValue` and large undo clear all folds (M1-07 keeps it simple - no edit-time adjustment).
- Visual fold marker: `▼` (expanded) / `▶` (collapsed) at the gutter level.

---

## 16. Find / Replace Widget

The editor includes a built-in find/replace **widget overlay** that paints in the top-right corner of the element. It's a self-contained UI; you just toggle it and the editor handles input routing.

### 16.1 Lifecycle

| Method | Signature | Description |
|--------|-----------|-------------|
| `openFindWidget([replaceMode])` | `(int?) -> void` | Open. `replaceMode=0` (default) for find-only, `1` for find+replace. |
| `closeFindWidget()` | `() -> void` | Close. |
| `isFindWidgetVisible()` | `() -> int` | Query. |
| `isFindWidgetReplaceMode()` | `() -> int` | Query. |

### 16.2 Input routing

While the widget is visible, key events flow into the widget's text fields instead of the buffer. The widget consumes:

- `Enter` / `Shift+Enter` - submit next/prev
- `Tab` - swap focused field
- `Backspace` - delete in field
- `Escape` - close widget
- Printable chars - append to field

If your host wants to provide its own UI instead of the built-in widget, leave `openFindWidget` unbound and call `findSet` / `findNext` / `findReplaceAll` directly.

### 16.3 Field state

| Method | Signature | Description |
|--------|-----------|-------------|
| `getFindWidgetField()` | `() -> int` | `0` = find field focused, `1` = replace field. |
| `getFindWidgetPattern()` | `() -> string` | Current find pattern. |
| `getFindWidgetReplacement()` | `() -> string` | Current replace pattern. |
| `findWidgetTypeChar(s)` | `(string) -> void` | Append to active field (called automatically by key handler). |
| `findWidgetBackspace()` | `() -> void` | Delete last char in active field. |
| `findWidgetSwitchField()` | `() -> void` | Toggle find <-> replace focus. |
| `findWidgetSubmitNext()` | `() -> void` | Advance to next match (Enter). |
| `findWidgetSubmitPrev()` | `() -> void` | Step back (Shift+Enter). |
| `findWidgetReplaceCurrent()` | `() -> void` | Replace current + advance. |
| `findWidgetReplaceAll()` | `() -> void` | Replace all matches. |

These low-level methods let you build a custom widget UI driven by the same state.

---

## 17. Minimap

A vertical strip painted at the right edge of the element. Each buffer line renders as a horizontal stroke whose length is proportional to the line's column count (capped at strip width). An overlay rectangle highlights the viewport-visible lines, doubling as a scrollbar.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setMinimapVisible(b)` | `(bool) -> bool` | Show / hide. Default: hidden. Accepts int **or** boolean: `0`/`false` hides, non-zero/`true` shows. |
| `getMinimapVisible()` | `() -> bool` | Query. |
| `setMinimapWidth(px)` | `(int) -> void` | Strip width in pixels. Default: `80`. |
| `getMinimapWidth()` | `() -> int` | Query. |

> **Visibility argument forms (KUI-029).** All `setXxxVisible` toggles accept **both** an int and a boolean. C truthiness applies to the int form (`0` = hide, non-zero = show) — unlike Lua, where `0` is truthy. Prefer the boolean form (`true`/`false`) for clarity. The matching `getXxxVisible()` getters return a **boolean**.

Mouse interaction (KUI-033):

- **Click** inside the strip scrolls the main viewport so the clicked line lands at the strip-relative center, then continues straight into a drag.
- **Press-and-drag** on the viewport overlay rectangle grabs it like a scrollbar thumb — the viewport follows the pointer proportionally until the button is released (VS Code / Sublime semantics). Leaving the element mid-drag does not cancel; the next mouse-up anywhere ends the drag.

```lua
ed:setMinimapVisible(true)   -- or setMinimapVisible(1)
ed:setMinimapWidth(120)
ed:setMinimapVisible(false)  -- or setMinimapVisible(0); hides (0 is honoured, KUI-029)
```

### 17.1 Programmatic scrolling (KUI-033)

Host-side scroll control, independent of any scrollbar form. Use these to restore a session's scroll position, center a search hit, or drive a custom scroll UI.

| Method | Signature | Description |
|--------|-----------|-------------|
| `scrollToLine(line[, position])` | `(int[, string]) -> float` | Scroll so 0-based `line` lands at `"top"`, `"center"` (default) or `"bottom"` of the viewport. Out-of-range lines are clamped. Returns the resulting `scrollTop` in px. |
| `getFirstVisibleLine()` | `() -> int` | 0-based index of the first line whose top edge is at/below the viewport top. |
| `getScrollTop()` | `() -> float` | Current scroll offset in px. |
| `setScrollTop(px)` | `(number) -> float` | Set the scroll offset; clamped to `[0, maxScroll]`. Returns the clamped value. |

```lua
ed:scrollToLine(120)            -- center line 120
ed:scrollToLine(0, "top")       -- jump to top
local line = ed:getFirstVisibleLine()
local px   = ed:getScrollTop()
ed:setScrollTop(px)             -- restore later
```

### 17.2 Scrollbar styles (KUI-034)

The editor supports two scrollbar forms which can be combined: the minimap (above) and a **classic vertical scrollbar** (track + draggable thumb with a guaranteed minimum thumb height, hugging the right edge). Thumb drag, track-click paging (towards the click, then draggable immediately) and hover/active tinting follow platform conventions.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setScrollbarStyle(s)` | `(string) -> string` | `"minimap"` (default) / `"classic"` / `"both"` / `"none"`. Case-insensitive; unknown keywords keep the current style. Returns the effective style. |
| `getScrollbarStyle()` | `() -> string` | Query. |
| `setScrollbarWidth(px)` | `(int) -> int` | Classic bar width, clamped to `[4, 40]`. Default: `12`. |
| `getScrollbarWidth()` | `() -> int` | Query. |

Notes:

- `"minimap"` preserves pre-KUI-034 behaviour exactly — the minimap (when `setMinimapVisible(true)`) is the only scrollbar.
- `"both"` arranges them like VS Code: the classic bar hugs the right edge, the minimap sits immediately to its left.
- The classic bar is omitted automatically when the buffer fits the viewport (nothing to scroll).
- Theme slots (each built-in flavor provides defaults): `scrollbarTrack`, `scrollbarThumb`, `scrollbarThumbHover`, `scrollbarThumbActive`.

```lua
ed:setScrollbarStyle("classic")  -- conventional desktop scrollbar only
ed:setScrollbarStyle("both")     -- VS Code arrangement
ed:setScrollbarWidth(14)
```

---

## 18. LSP Integration

The editor ships a **full LSP client** (M2-14). You start a language server process, attach the editor to a file URI, and the editor auto-syncs `didOpen` / `didChange` / `didClose` notifications. Eight feature requests are supported with a polling-style result API.

### 18.1 Lifecycle

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspStart(args...)` | `(string, ...) -> int` | Spawn the server process. First arg = path, rest = command-line arguments. Returns `1` on success. |
| `lspStop()` | `() -> int` | Send `shutdown` + `exit`, then terminate the process. |
| `lspIsRunning()` | `() -> int` | Process is alive. |
| `lspIsInitialized()` | `() -> int` | Server has responded to `initialize`. |
| `lspState()` | `() -> string` | One of `Uninitialized` / `Initializing` / `Initialized` / `ShuttingDown` / `Shutdown` / `Exited`. |
| `lspTick([maxBytes])` | `(int?) -> void` | Pump pending I/O. Call this from your host's main loop or attach to `onTick`. |
| `lspLastError()` | `() -> string` | Most recent error from the client. |

### 18.2 Document sync

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspDidOpen(uri, languageId, text)` | `(s,s,s) -> void` | Notify server that a file is open. |
| `lspDidChangeFull(uri, text)` | `(s,s) -> void` | Push full content (no incremental sync yet). |
| `lspDidClose(uri)` | `(s) -> void` | Notify server. |
| `lspIsOpen(uri)` | `(s) -> int` | Query. |
| `lspVersionOf(uri)` | `(s) -> int` | Server-side version counter. |

### 18.3 Auto-attach (recommended)

Instead of manually managing `didOpen` / `didChange`, attach the editor to a URI and the editor auto-syncs on every `onTick` whenever the buffer mutates:

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspAttach(uri, languageId)` | `(s,s) -> void` | Send `didOpen`, start auto-sync. |
| `lspDetach()` | `() -> void` | Send `didClose`, stop auto-sync. |
| `lspGetAttachedUri()` | `() -> string` | Current URI, or `""` if not attached. |

```lua
-- Start clangd
ed:lspStart("/usr/bin/clangd", "--background-index")

-- Wait until initialized (poll from your tick loop):
local function tryAttach()
    if ed:lspIsInitialized() == 1 then
        ed:lspAttach("file:///tmp/foo.cpp", "cpp")
        return
    end
    setTimeout(tryAttach, 100)
end
tryAttach()
```

### 18.4 Diagnostics

The server publishes `textDocument/publishDiagnostics`; the editor stores them and paints squiggles + gutter markers automatically.

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspDiagnosticsJson(uri)` | `(s) -> string` | All diagnostics for `uri` as a JSON array. |
| `lspDiagnosticCountsJson([uri])` | `(s?) -> string` | Counts `{error, warning, info, hint}` (optionally per uri). |
| `lspInjectPublishDiagnosticsJson(json)` | `(s) -> void` | Debug helper: inject fake diagnostics without a real server. |

### 18.5 Feature requests (polling API)

All request methods return a poll handle (integer); use `lspIsPollPending` / `lspPollResultJson` to retrieve the response:

| Method | Args | Returns (via poll) |
|--------|------|-------------------|
| `lspRequestHover` | `(uri, line, col)` | JSON: `{contents: ...}` |
| `lspRequestCompletion` | `(uri, line, col, [trigger])` | JSON: completion items |
| `lspRequestDefinition` | `(uri, line, col)` | JSON: locations |
| `lspRequestReferences` | `(uri, line, col, [includeDecl])` | JSON: locations |
| `lspRequestSignatureHelp` | `(uri, line, col)` | JSON: signature info |
| `lspRequestFormatting` | `(uri, [tabSize, insertSpaces])` | JSON: text edits |
| `lspRequestRename` | `(uri, line, col, newName)` | JSON: workspace edits |
| `lspRequestPrepareRename` | `(uri, line, col)` | JSON: rename range |
| `lspRequest` | `(method, [paramsJson])` | JSON: raw LSP `result` — generic pass-through, see §18.7 |

### 18.6 Poll API

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspIsPollPending(handle)` | `(int) -> int` | `1` if not yet resolved. |
| `lspPollResultJson(handle)` | `(int) -> string` | JSON result on success, `""` if pending. |
| `lspPollErrorJson(handle)` | `(int) -> string` | JSON error on failure. |

```lua
local h = ed:lspRequestHover("file:///tmp/foo.cpp", 10, 5)

local function check()
    if ed:lspIsPollPending(h) == 1 then
        setTimeout(check, 50)
        return
    end
    local err = ed:lspPollErrorJson(h)
    if err ~= "" then print("hover error:", err) return end
    local res = ed:lspPollResultJson(h)
    showHoverTooltip(res)
end
check()
```

### 18.7 Generic pass-through (`lspRequest`)

`lspRequest(method, [paramsJson])` sends *any* LSP request by method name with raw JSON params — the escape hatch for server features that have no dedicated wrapper in §18.5, e.g. `workspace/symbol`, `textDocument/documentSymbol`, `textDocument/prepareCallHierarchy` + `callHierarchy/incomingCalls` / `callHierarchy/outgoingCalls`, `textDocument/semanticTokens/full`, `textDocument/foldingRange`, …

| Method | Signature | Description |
|--------|-----------|-------------|
| `lspRequest(method, [paramsJson])` | `(s, s?) -> int` | Poll handle (`>0`), or `0` when rejected. |

Rules:

* `paramsJson` must be a JSON **object**, **array** or `null`. Omitting the argument (or passing `""` / `nil`) sends `null` params.
* The lifecycle methods `initialize`, `shutdown` and `exit` are **rejected** (returns `0`) — they are owned by `lspStart` / `lspStop`, and sending them out-of-band would desync the client state machine. `$/...` methods (e.g. `$/cancelRequest`) are allowed.
* `0` is also returned when the server is not initialized or `paramsJson` does not parse; `lspLastError()` carries the reason.
* Responses are consumed through the **same poll API as §18.6** — zero new plumbing: `lspIsPollPending` / `lspPollResultJson` / `lspPollErrorJson`.

```lua
-- Workspace-wide symbol search via generic pass-through.
local h = ed:lspRequest("workspace/symbol", '{"query":"EditorState"}')
if h == 0 then
    print("lspRequest rejected:", ed:lspLastError())
    return
end

local function check()
    if ed:lspIsPollPending(h) == 1 then
        setTimeout(check, 50)
        return
    end
    local err = ed:lspPollErrorJson(h)
    if err ~= nil and err ~= "" then print("symbol error:", err) return end
    local res = ed:lspPollResultJson(h)
    -- res is the raw LSP result: a JSON array of SymbolInformation.
    print("symbols:", res)
end
check()
```

---

## 19. Git Diff Gutter & Blame

### 19.1 Git diff gutter (M3-01)

Color-coded gutter strip showing added / modified / deleted lines. You feed the diff in via Lua; the editor paints it.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setGitDiff(addedJson, modifiedJson, deletedAfterJson)` | `(s,s,s) -> void` | Set all three sets at once. JSON arrays of line numbers (deleted = "after this line"). |
| `setGitAddedLines(json)` | `(s) -> void` | Just added lines. |
| `setGitModifiedLines(json)` | `(s) -> void` | Just modified. |
| `setGitDeletedAfterLines(json)` | `(s) -> void` | Just deletions. |
| `clearGitDiff()` | `() -> void` | Drop the diff. |
| `setGitGutterVisible(b)` | `(bool) -> bool` | Master toggle. Accepts int **or** boolean (`0`/`false` hides; see KUI-029 note in §17). |
| `getGitGutterVisible()` | `() -> bool` | Query. |
| `getGitDiffJson()` | `() -> string` | All three sets as JSON. |
| `getGitDiffCounts()` | `() -> string` | `"added,modified,deleted"` count triple. |
| `getGitKindAtLine(line)` | `(int) -> int` | `0`=clean, `1`=added, `2`=modified. |
| `hasGitDeletedAfter(line)` | `(int) -> int` | `1` if deletion marker after this line. |

```lua
ed:setGitDiff(
    "[10, 11, 12]",       -- added
    "[20]",                -- modified
    "[30]"                 -- deleted after line 30
)
```

### 19.2 Inline blame (M3-02)

Shows commit / author / date as a virtual line annotation at end-of-line. Supports per-line (legacy) and per-document modes.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setBlameLine(line, hash, author, dateStr)` | `(i,s,s,s) -> void` | Per-line set. |
| `setBlameRecordsJson(json)` | `(s) -> void` | Bulk set all records (modern path). |
| `clearBlame()` | `() -> void` | Drop all. |
| `setBlameMode(mode)` | `(int\|s) -> bool` | `0`/`"off"` = off, `1`/`"caret"` = current line only, `2`/`"all"` = all lines. String aliases are case-insensitive. Returns whether the mode was accepted. (KUI-047: now accepts both the documented int and the string aliases.) |
| `getBlameMode()` | `() -> int` | Query — returns `0`/`1`/`2` (off/caret/all). |
| `getBlameRecordCount()` | `() -> int` | Query. |
| `getBlameLineJson(line)` | `(i) -> string` | Per-line record. |
| `getBlameRecordsJson()` | `() -> string` | All records. |
| `getBlameLineComposed(line)` | `(i) -> string` | Composed text shown in the gutter (`"abc123 - Alice, 2026-05-01"`). |

---

## 20. Sticky Scroll, Brackets & Indent Guides

### 20.1 Sticky scroll (M3-03)

When the user scrolls down past the start of a function/class/method, the line containing the function header "sticks" at the top of the viewport so context never disappears.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setStickyVisible(b)` | `(bool) -> bool` | Toggle. Default: on if language is set. Accepts int **or** boolean (`0`/`false` hides; see KUI-029 note in §17). |
| `getStickyVisible()` | `() -> bool` | Query. |
| `setStickyMaxDepth(n)` | `(int) -> void` | Max nested stuck lines. Default `5`. |
| `getStickyMaxDepth()` | `() -> int` | Query. |
| `getStickyLinesJson(scrollLine)` | `(int) -> string` | Currently-stuck line indices at the given top-of-viewport. |
| `getStickyLineCount(scrollLine)` | `(int) -> int` | Count. |

### 20.2 Bracket-pair colorization (M3-04)

Colors `()` `[]` `{}` by nesting depth using the theme's `bracketColors[6]` palette.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setBracketColorsEnabled(b)` | `(int) -> void` | Toggle. |
| `getBracketColorsEnabled()` | `() -> int` | Query. |
| `getBracketDepthAt(line, col)` | `(i,i) -> int` | Nesting depth at a position. |
| `getBracketCount()` | `() -> int` | Total bracket pairs detected. |

### 20.3 Indent guides

Vertical light-grey lines drawn at each indentation level. The active one (column of the caret's indent) is highlighted.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setIndentGuidesEnabled(b)` | `(int) -> void` | Toggle. |
| `getIndentGuidesEnabled()` | `() -> int` | Query. |
| `getIndentGuideColumnCount(line)` | `(i) -> int` | Number of guides drawn on the given line. |

---

## 21. Breadcrumb

Displays the symbol-path from buffer root to the current cursor position (e.g. `class Foo > method bar > if-block`). Useful as a header above the editor.

| Method | Signature | Description |
|--------|-----------|-------------|
| `getBreadcrumbJson(line, [maxChars])` | `(i, i?) -> string` | JSON of segments at the given line. Each segment: `{kind, name, line, col}`. |
| `getBreadcrumbText(line, [maxChars])` | `(i, i?) -> string` | Pre-composed text like `"Foo > bar > if-block"`. |
| `getBreadcrumbCount(line, [maxChars])` | `(i, i?) -> int` | Segment count. |

```lua
print(ed:getBreadcrumbText(ed:getCaretLine()))
-- "MyClass > method() > if-block"
```

---

## 22. Snippets

LSP-style snippet expansion with placeholders, choices, mirrors, and variables.

### 22.1 Snippet syntax

```
${1:placeholder}      # tab stop 1, default text "placeholder"
${2}                  # tab stop 2, empty default
$0                    # final caret position (always last)
${1|red,green,blue|}  # tab stop 1 with choices
${VAR_NAME}           # variable reference (see §22.4)
\$\\                  # literal $ and \ via escaping
```

Multiple `$1` references in one snippet become **mirrors**: editing the primary updates all mirrors live.

### 22.2 Driver methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `insertSnippet(template)` | `(s) -> int` | Parse + insert. Returns `1` on success, `0` on parse error. |
| `snippetNext()` | `() -> int` | Advance to next tab stop. Returns `1` if a stop is now active. |
| `snippetPrev()` | `() -> int` | Step back. |
| `snippetCancel()` | `() -> void` | Exit snippet mode. |
| `snippetIsActive()` | `() -> int` | Query. |
| `snippetActiveIndex()` | `() -> int` | Current tab stop number (1..N, or 0 if at `$0`). |
| `snippetStopCount()` | `() -> int` | Total tab stops. |
| `snippetMirrorCount()` | `() -> int` | Total mirror sites. |

### 22.3 Choices

When the active tab stop is a `${N|opt1,opt2|}`, query options for a UI dropdown:

| Method | Signature | Description |
|--------|-----------|-------------|
| `snippetChoiceCount()` | `() -> int` | Options for current tab stop (0 if not a choice stop). |
| `snippetChoiceAt(i)` | `(i) -> string` | Option `i`. |

### 22.4 Variable context

LSP snippet variables like `$TM_FILENAME`, `$CLIPBOARD`, `$CURRENT_YEAR` need application context. Set them before `insertSnippet`:

| Method | Signature | Description |
|--------|-----------|-------------|
| `setSnippetContextField(name, value)` | `(s,s) -> void` | Set one variable. |
| `clearSnippetContext()` | `() -> void` | Clear all. |

```lua
ed:setSnippetContextField("TM_FILENAME", "main.cpp")
ed:setSnippetContextField("CURRENT_YEAR", "2026")

ed:insertSnippet("// $CURRENT_YEAR ${1:author}\n#include <iostream>\n$0")
-- caret on the "author" tab stop, ready for typing
```

---

## 23. Ghost Text (Inline Suggestions)

Ghost text is greyed-out preview text rendered after the caret without committing it to the buffer. The user accepts it (`Tab`), partially accepts it (word-by-word), or dismisses it (`Esc` / any caret movement). This is the foundation for AI / Copilot-style inline suggestions.

| Method | Signature | Description |
|--------|-----------|-------------|
| `setGhostText(s)` | `(string) -> void` | Set / replace the ghost text at the current caret position. |
| `clearGhostText()` | `() -> void` | Drop the ghost text. |
| `acceptGhostText()` | `() -> int` | Accept full ghost text (insert into buffer). Returns `1` on success. |
| `acceptGhostWord()` | `() -> int` | Accept just the next word/whitespace chunk. |
| `hasGhostText()` | `() -> int` | Query. |
| `getGhostText()` | `() -> string` | Current ghost text. |

### 23.1 Lifecycle

1. Application requests a suggestion (e.g. from an AI service).
2. Application calls `ed:setGhostText("...")` once the response arrives.
3. Editor paints the text in 50%-alpha grey, starting at the caret.
4. User behavior:
   - `Tab` -> `acceptGhostText()` (consumed by key handler)
   - Type any printable -> ghost is dismissed (the typed char goes into the buffer normally)
   - Move caret -> ghost is dismissed silently
   - `Escape` -> ghost is dismissed silently

The ghost text is **anchored to a specific (line, col)** at the moment of `setGhostText`. If the caret moves away from that anchor, the ghost auto-dismisses on the next paint.

### 23.2 Multi-line ghost

Ghost text containing `\n` paints across multiple lines. The first line continues from the caret; subsequent lines start at column 0 of the next visual line.

```lua
ed:setGhostText("    return value;\n}")
```

### 23.3 Word-by-word acceptance

`acceptGhostWord()` peels off the leading word + whitespace and accepts that prefix only. This lets users incrementally accept long suggestions:

```lua
-- Suggestion: "for (int i = 0; i < n; ++i) {"
-- Each call consumes one identifier + trailing whitespace
ed:acceptGhostWord()    -- inserts "for "
ed:acceptGhostWord()    -- inserts "(int "
ed:acceptGhostWord()    -- inserts "i "
-- ...etc
```

Bind this to `Ctrl+RightArrow` in your application if you want VS Code-style word acceptance.

---

## 24. Line Transforms & Navigation

### 24.1 Whole-buffer transforms (M4-04)

Each method operates on the whole buffer (or selection's lines if a selection is active). Returns `1` if the buffer changed.

| Method | Signature | Description |
|--------|-----------|-------------|
| `transformToUpperCase()` | `() -> int` | Selection or buffer to upper case. |
| `transformToLowerCase()` | `() -> int` | To lower case. |
| `transformToTitleCase()` | `() -> int` | To title case (first letter of each word capitalized). |
| `trimTrailingWhitespace()` | `() -> int` | Strip trailing whitespace from every line. |
| `joinLines()` | `() -> int` | Join all lines (or selection's lines) with single space. |
| `joinLinesWithSeparator(sep)` | `(string) -> int` | Same with custom separator. |
| `removeDuplicateLines()` | `() -> int` | Drop adjacent duplicate lines. |
| `sortLinesAscending()` | `() -> int` | Sort case-sensitive. |
| `sortLinesDescending()` | `() -> int` | Reverse sort. |
| `sortLinesCaseInsensitiveAscending()` | `() -> int` | Sort case-insensitive. |
| `sortLinesCaseInsensitiveDescending()` | `() -> int` | Reverse case-insensitive. |
| `sortLinesByLength()` | `() -> int` | By byte count ascending. |
| `sortLinesByLengthDescending()` | `() -> int` | Reverse. |
| `shuffleLines([seed])` | `(int?) -> int` | Random permutation. Optional `seed` for reproducible shuffles. |

### 24.2 Line operations (M1-12)

| Method | Signature | Description |
|--------|-----------|-------------|
| `toggleLineComment()` | `() -> int` | Add/remove `//` (or `#` / `--` depending on language). Bound to `Ctrl+/`. |
| `moveLineUp()` | `() -> int` | Bound to `Alt+ArrowUp`. |
| `moveLineDown()` | `() -> int` | Bound to `Alt+ArrowDown`. |
| `copyLineUp()` | `() -> int` | Bound to `Shift+Alt+ArrowUp`. |
| `copyLineDown()` | `() -> int` | Bound to `Shift+Alt+ArrowDown`. |
| `deleteLine()` | `() -> int` | Bound to `Ctrl+Shift+K`. |

### 24.3 Navigation (M4-04)

| Method | Signature | Description |
|--------|-----------|-------------|
| `gotoLine(line, [col])` | `(i, i?) -> int` | Jump to line (1-based per VS Code convention; the editor converts to 0-based internally). Returns `1` on success. |
| `gotoBufferStart()` | `() -> int` | Jump to `(0, 0)`. |
| `gotoBufferEnd()` | `() -> int` | Jump to last line, last column. |

```lua
ed:gotoLine(42)         -- jump to line 42
ed:gotoLine(42, 10)     -- line 42, column 10
ed:gotoBufferEnd()
```

---

## 25. Document Symbols

The editor maintains a cache of document symbols (functions, classes, variables) typically populated from LSP `textDocument/documentSymbol` responses. Used by goto-symbol UI and the breadcrumb.

### 25.1 Set / clear

| Method | Signature | Description |
|--------|-----------|-------------|
| `setDocumentSymbolsJson(json)` | `(s) -> void` | Bulk set. JSON: array of `{name, kind, range, selectionRange, detail?, container?, children?}`. |
| `clearDocumentSymbols()` | `() -> void` | Drop the cache. |
| `getDocumentSymbolCount()` | `() -> int` | Total flattened count. |

### 25.2 Per-symbol accessors

All take a 0-based index `i` into the flattened symbol list:

| Method | Signature | Returns |
|--------|-----------|---------|
| `getDocumentSymbolName(i)` | `(int) -> string` | Symbol name |
| `getDocumentSymbolKind(i)` | `(int) -> int` | LSP `SymbolKind` (1=File, 5=Class, 12=Function, ...) |
| `getDocumentSymbolKindName(i)` | `(int) -> string` | `"Function"` etc. |
| `getDocumentSymbolDepth(i)` | `(int) -> int` | Nesting depth (0 = top-level) |
| `getDocumentSymbolStartLine(i)` | `(int) -> int` | Range start (LSP UTF-16 line, 0-based) |
| `getDocumentSymbolStartCol(i)` | `(int) -> int` | Range start col |
| `getDocumentSymbolEndLine(i)` | `(int) -> int` | Range end |
| `getDocumentSymbolEndCol(i)` | `(int) -> int` | Range end col |
| `getDocumentSymbolDetail(i)` | `(int) -> string` | Optional detail (e.g. function signature) |
| `getDocumentSymbolContainer(i)` | `(int) -> string` | Container name (parent class/method) |

### 25.3 LSP integration

```lua
-- After lspAttach, request symbols periodically:
local h = ed:lspRequestSomeSymbolsApi(...)  -- application-defined
ed:setDocumentSymbolsJson(ed:lspPollResultJson(h))

-- Build a "Goto Symbol" picker:
local n = ed:getDocumentSymbolCount()
for i = 0, n - 1 do
    print(ed:getDocumentSymbolKindName(i),
          ed:getDocumentSymbolName(i),
          ed:getDocumentSymbolStartLine(i))
end
```

---

## 26. Fuzzy Picker

A reusable session-based picker that powers Quick Open, Command Palette, Goto Symbol, etc. The application provides items + an active query; the editor scores and orders them.

### 26.1 Session lifecycle

| Method | Signature | Description |
|--------|-----------|-------------|
| `pickerBegin()` | `() -> void` | Start a new picker session. Drops any prior session. |
| `pickerAddItem(id, label)` | `(s, s) -> void` | Add one item. `id` is opaque; `label` is what's matched against the query. |
| `pickerSetQuery(q)` | `(s) -> void` | Set the user's current input. Triggers re-scoring. |
| `pickerEnd()` | `() -> void` | End the session, freeing resources. |

### 26.2 Result accessors

Items are sorted by descending score (best match first). Use these to render a list UI:

| Method | Signature | Description |
|--------|-----------|-------------|
| `pickerCount()` | `() -> int` | Total items (filtered by query). |
| `pickerIdAt(i)` | `(int) -> string` | Application-defined id. |
| `pickerLabelAt(i)` | `(int) -> string` | Display label. |
| `pickerScoreAt(i)` | `(int) -> int` | Match score (higher is better). |
| `pickerActiveIndex()` | `() -> int` | Currently highlighted item (for Up/Down navigation). |
| `pickerSetActiveIndex(i)` | `(int) -> void` | Move highlight. |
| `pickerActiveId()` | `() -> string` | Convenience: id of active item. |
| `pickerIsOpen()` | `() -> int` | Session active. |

### 26.3 Pattern: Goto Symbol picker

```lua
ed:pickerBegin()
local n = ed:getDocumentSymbolCount()
for i = 0, n - 1 do
    local id    = tostring(ed:getDocumentSymbolStartLine(i))
    local kind  = ed:getDocumentSymbolKindName(i)
    local name  = ed:getDocumentSymbolName(i)
    ed:pickerAddItem(id, kind .. " " .. name)
end

-- Bind input to query:
inputField:on("input", function(text)
    ed:pickerSetQuery(text)
    refreshPickerUi()
end)

-- Bind Enter key:
function commitPicker()
    local lineStr = ed:pickerActiveId()
    ed:gotoLine(tonumber(lineStr) + 1)
    ed:pickerEnd()
end
```

### 26.4 Scoring algorithm

The picker uses a fuzzy matcher similar to fzf:

- Sequential character matches in the label score positively.
- Bonuses for: word starts (after `_` / `.` / `/` / camelCase), match at start, consecutive matches.
- Penalties for: non-matched chars between matches.
- Case-insensitive by default; if the query contains any uppercase, becomes case-sensitive (smart-case).

Items with score < threshold are filtered out (drop from `pickerCount`).

---

## 27. Built-in Command Registry

The editor also exposes a built-in catalog of commands - the same operations bound to default shortcuts but reachable by name. Use this to populate a Command Palette.

| Method | Signature | Description |
|--------|-----------|-------------|
| `getCommandCount()` | `() -> int` | Total registered commands. |
| `getCommandIdAt(i)` | `(int) -> string` | Stable id like `"editor.action.toggleLineComment"`. |
| `getCommandTitleAt(i)` | `(int) -> string` | Human-readable title like `"Toggle Line Comment"`. |
| `getCommandCategoryAt(i)` | `(int) -> string` | Group like `"Edit"`, `"Selection"`, `"View"`, `"LSP"`. |

```lua
-- Build a Command Palette:
ed:pickerBegin()
local n = ed:getCommandCount()
for i = 0, n - 1 do
    local id    = ed:getCommandIdAt(i)
    local title = ed:getCommandTitleAt(i)
    local cat   = ed:getCommandCategoryAt(i)
    ed:pickerAddItem(id, cat .. ": " .. title)
end
```

Dispatching a chosen command id back to the editor is currently the application's responsibility (a `dispatchCommand(id)` method is on the M5 roadmap).

---

## 28. Events & Versioning

The editor does **not** dispatch DOM events for buffer changes (no `change` / `cursor-change` event). Instead it exposes two version counters and the application **polls** them.

### 28.1 Version counters

| Counter | Bumps on |
|---------|----------|
| `getVersion()` | Any state change: buffer / caret / selection / scroll / focus / blink / IME |
| `getBufferContentVersion()` | Buffer mutations only: typing / paste / undo / redo / setValue |

Use `getBufferContentVersion()` for "did the document change" checks (saves), and `getVersion()` for "did anything visible change" checks (caret position display, status bar update).

### 28.2 Polling pattern

```lua
local lastBC = ed:getBufferContentVersion()
local lastV  = ed:getVersion()

setInterval(function()
    local bc = ed:getBufferContentVersion()
    if bc ~= lastBC then
        lastBC = bc
        markDirty()                          -- "*" marker on tab title
    end
    local v = ed:getVersion()
    if v ~= lastV then
        lastV = v
        statusBar:setText(string.format("Ln %d, Col %d",
                                         ed:getCaretLine() + 1,
                                         ed:getCaretCol() + 1))
    end
end, 50)   -- 20 Hz polling
```

### 28.3 Why polling, not events?

Three reasons:

1. **Determinism**: Tests can call `getVersion()` after a synchronous Lua command without race conditions or pending events.
2. **Throttling**: At 60 Hz the editor produces 60 caret-blink version bumps per second; events would flood the bus. Polling at the application's natural cadence is much cheaper.
3. **DLL ABI**: Events flowing across the DLL boundary would require Lua-side callback handles (HOST-016, not yet implemented). Version counters work with the typed-value channel today.

### 28.4 Future: callback handles (M5+)

The `HOST-016` proposal will let the application register a Lua function that fires synchronously after every buffer mutation. Until it lands, polling is the supported way.

---

## 29. Performance Notes

### 29.1 Repaint contract and partial repaint

Since KUI extension API v1.11 the editor declares the **repaint contract** (`kuiext_paintContract()` returns `KUI_EXT_PAINT_CONTRACT_REPAINT_ON_CHANGE`, see [`extension_repaint_contract.md`](extension_repaint_contract.md)): every path that changes what the editor paints requests a repaint, so a page hosting `<kui-editor>` keeps the host's small-change frames (hover / focus / typing elsewhere repaint only what changed) and the editor's own requests become exact damage.

To make that guarantee hold, keystrokes now repaint the **whole editor element** (in the contract that is the editor only, not the page). The former M5-04 single-line `invalidateRect` was unsafe once the host stopped re-recording the whole page: one keystroke can also change other lines or regions (occurrence highlight, bracket match / colorization depth, multi-line tree-sitter tokens, minimap, horizontal scrollbar, multi-line ghost text, snippet mirrors, wrapped rows). The caret blink still uses a partial repaint: the caret line's full-width band in non-wrap mode (the whole editor in word-wrap mode). LSP diagnostics landing, find-widget keys, and focus gain / loss also request repaints.

The instrumentation is exposed for testing:

| Method | Signature | Description |
|--------|-----------|-------------|
| `paintInstrSnapshot()` | `() -> string` | `"<full>,<partial>,<lastX>,<lastY>,<lastW>,<lastH>"` counts + last partial rect. |
| `paintInstrReset()` | `() -> void` | Reset all counters. |
| `nfr005DllScanBench(buf)` | `(s) -> int` | Internal scan benchmark (memchr+memcmp baseline). |

With the contract, typing reports full (whole-element) repaints; caret blinks report partial ones.

### 29.2 Tree-sitter incremental parsing

The parser keeps the previous `TSTree` and edits it via `ts_tree_edit` before re-parsing. Typical incremental parse on a 1 000-line C++ file: ~1 ms.

### 29.3 Tree-sitter buffer cap

For buffers larger than `kTsMaxBufferBytes` (~2 MB), tree-sitter is **disabled** for that element to keep paint under 16 ms. The fallback is the lightweight token regex (still gives basic keyword highlighting).

### 29.4 Find/replace caching

The match list is cached against `bufferContentVersion`. Calling `findNext()` after typing 100 chars without changing the pattern hits a fresh re-scan once, then 99 cache hits.

### 29.5 LSP I/O on a worker thread

The LSP client runs stdin/stdout I/O on a dedicated thread; only response handler dispatch runs on the main thread inside `lspTick()`. Calling `lspTick(maxBytes)` with a small budget (e.g. 64 KB) keeps frame time stable even under heavy server traffic.

### 29.6 Multi-cursor compound undo

When 100 cursors all delete one char each, the editor emits 100 `EditOp`s but groups them under one `batchId` so `Ctrl+Z` collapses them to one user-visible step. Undo replays in reverse order, restoring all cursor positions atomically.

---

## 30. Complete Examples

### 30.1 Minimal embed

```html
<kui-editor language="lua" theme="mocha" id="ed"></kui-editor>

<script type="text/lua">
    local ed = document:querySelector("#ed")
    ed:setValue([[
local function fact(n)
    if n <= 1 then return 1 end
    return n * fact(n - 1)
end
print(fact(10))
]])
end
</script>
```

### 30.2 Editor with status bar

```html
<style>
    #ed { height: 500px; width: 100%; }
    #status {
        font-family: Consolas, monospace;
        background: #1e1e3a; color: #ccc;
        padding: 4px 8px;
    }
</style>

<kui-editor id="ed" language="cpp"></kui-editor>
<div id="status">Ln 1, Col 1</div>

<script type="text/lua">
    local ed = document:querySelector("#ed")
    local sb = document:querySelector("#status")

    setInterval(function()
        local ln = ed:getCaretLine() + 1
        local col = ed:getCaretCol() + 1
        local total = ed:getLineCount()
        sb.textContent = string.format("Ln %d, Col %d (of %d)", ln, col, total)
    end, 100)
end
</script>
```

### 30.3 Editor with command palette (Ctrl+Shift+P)

```html
<kui-editor id="ed"></kui-editor>
<div id="palette" style="display:none; position:fixed; ...">
    <input id="palette-input">
    <ul id="palette-list"></ul>
</div>

<script type="text/lua">
    local ed       = document:querySelector("#ed")
    local palette  = document:querySelector("#palette")
    local input    = document:querySelector("#palette-input")
    local list     = document:querySelector("#palette-list")

    -- Define dispatchers for built-in commands
    local dispatch = {
        ["editor.action.toggleLineComment"] = function() ed:toggleLineComment() end,
        ["editor.action.deleteLine"]        = function() ed:deleteLine() end,
        ["editor.action.sortLinesAscending"]= function() ed:sortLinesAscending() end,
        -- ... map every command id to its action ...
    }

    local function openPalette()
        ed:pickerBegin()
        local n = ed:getCommandCount()
        for i = 0, n - 1 do
            ed:pickerAddItem(ed:getCommandIdAt(i),
                              ed:getCommandCategoryAt(i) .. ": "
                              .. ed:getCommandTitleAt(i))
        end
        palette.style.display = "block"
        input:focus()
        input.value = ""
        ed:pickerSetQuery("")
        renderList()
    end

    function renderList()
        list.innerHTML = ""
        local n = ed:pickerCount()
        for i = 0, n - 1 do
            local li = document:createElement("li")
            li.textContent = ed:pickerLabelAt(i)
            if i == ed:pickerActiveIndex() then
                li.style.background = "#3b82f6"
            end
            list:appendChild(li)
        end
    end

    input:addEventListener("input", function()
        ed:pickerSetQuery(input.value)
        renderList()
    end)

    input:addEventListener("keydown", function(e)
        if e.key == "ArrowDown" then
            ed:pickerSetActiveIndex(ed:pickerActiveIndex() + 1)
            renderList()
        elseif e.key == "ArrowUp" then
            ed:pickerSetActiveIndex(ed:pickerActiveIndex() - 1)
            renderList()
        elseif e.key == "Enter" then
            local id = ed:pickerActiveId()
            if dispatch[id] then dispatch[id]() end
            ed:pickerEnd()
            palette.style.display = "none"
            ed:focus()
        elseif e.key == "Escape" then
            ed:pickerEnd()
            palette.style.display = "none"
            ed:focus()
        end
    end)

    document:addEventListener("keydown", function(e)
        if e.ctrlKey and e.shiftKey and e.key == "P" then
            openPalette()
            e:preventDefault()
        end
    end)
end
</script>
```

### 30.4 Editor + clangd LSP

```lua
local ed = document:querySelector("kui-editor")
ed:setLanguage("cpp")

-- Spawn clangd
ed:lspStart("clangd", "--log=verbose", "--background-index")

-- Pump I/O every frame:
setInterval(function()
    ed:lspTick(8192)
end, 16)

-- After init, attach:
local function tryAttach()
    if ed:lspIsInitialized() == 0 then setTimeout(tryAttach, 100) return end
    ed:lspAttach("file:///tmp/main.cpp", "cpp")

    -- Periodic symbol refresh:
    setInterval(function()
        local h = ed:lspRequestSomeSymbolsApi("file:///tmp/main.cpp")
        setTimeout(function() pollSymbols(h) end, 200)
    end, 5000)
end
tryAttach()

-- Ctrl+. -> hover at caret
document:addEventListener("keydown", function(e)
    if not (e.ctrlKey and e.key == ".") then return end
    local h = ed:lspRequestHover("file:///tmp/main.cpp",
                                  ed:getCaretLine(),
                                  ed:getCaretCol())
    setTimeout(function() pollHover(h) end, 100)
end)
```

### 30.5 AI ghost-text integration

```lua
-- When the user pauses typing for 500 ms, query the AI service.
local pausedTimer
ed:addEventListener("keyup", function()
    if pausedTimer then clearTimeout(pausedTimer) end
    ed:clearGhostText()

    pausedTimer = setTimeout(function()
        local prefix = ed:getValue():sub(1, getCaretByteOffset(ed))
        fetchSuggestion(prefix, function(suggestion)
            if suggestion ~= "" then
                ed:setGhostText(suggestion)
            end
        end)
    end, 500)
end)
```

---

## 31. Troubleshooting

### Editor element is empty / not rendering

- Verify `kui_ext_editor.dll` is in your host's extensions directory. Check stderr for `[EXT] Loaded extension kui_ext_editor v1.9.0`.
- Verify host's API version is >= 9: `kuiext_init` rejects older hosts with `KUI_EXT_ERR_VERSION` (-2).
- Check the element has non-zero size. If wrapped in a flex container, give it `flex: 1` or explicit `height`.

### Syntax highlighting not working

- Did you set `language="cpp"` (or via `setLanguage`)? Without a language, only the lightweight fallback tokenizer runs.
- Check the language name is in [§6.1](#61-supported-languages). `setLanguage("rust")` silently disables highlighting (Rust grammar not bundled).
- For very large buffers (> 2 MB), tree-sitter is intentionally disabled. Check `getTsDebugInfo()`.

### Find/replace returns 0 matches for a regex

- Did you set the `useRegex=16` flag? Plain text mode treats the pattern as a literal.
- Check `findGetError()` for PCRE2 compile errors.
- Remember that `^` / `$` only work multi-line if you set `multiline=32`.

### Ctrl+C copies but not selection

- Default behavior: empty selection -> copy current line + `\n` (VS Code parity). To get the old "do nothing" behavior, check `ed:hasSelection() == 0` first and skip.

### LSP server starts but never initializes

- Call `lspTick()` regularly (e.g. every 16 ms). Without it, no I/O is pumped.
- Check `lspState()` for the server's lifecycle state.
- Check `lspLastError()` for client-side errors.
- Check the server's stderr for crashes (most servers also log to a `.log` file - clangd uses `--log=verbose`).

### Diagnostics squiggles don't appear

- Did you `lspAttach(uri, languageId)`? Without attach, the editor doesn't know which file's diagnostics to render.
- Verify with `lspDiagnosticsJson(uri)` - if it returns `[]`, the server hasn't published any.
- Inject test diagnostics: `lspInjectPublishDiagnosticsJson('[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}},"message":"test","severity":1}]')`.

### Multi-cursor changes don't all undo with one Ctrl+Z

- This was fixed in M1-CARET-C-fix. Verify your DLL is >= 1.7.0.
- The `paintInstrSnapshot` / `getCursorCount` methods can verify cursor count before / after the edit.

### Editor crashes the host

- The host's SEH guard wraps every callback. A real crash should log `[EXT][SEH] crash in <kui-editor>::onPaint: 0xC0000005`.
- If the host actually dies (no SEH log), it's likely a host-side bug. File an issue with the stack trace.

---

## 32. FAQ

### Q: Can I use `<kui-editor>` for a single-line input box?

**A:** Not really. The editor is optimized for multi-line code. For single-line input, use `<input type="text">` or `<textarea>` from the host's built-in elements.

### Q: How do I save the buffer to disk?

**A:** The editor has no file I/O - it's just a text widget. Get the content with `ed:getValue()` and write it from your application code (host C++, or Lua via `io.open(path, "w"):write(content)`).

### Q: Can I disable the gutter?

**A:** Currently no Lua API exposes this; the gutter is always shown when `state.showGutter` is `true` (default). A `setGutterVisible(b)` method is on the M5 roadmap. Workaround: set `theme.gutterBackground` to match `theme.background` so the gutter is invisible.

### Q: How do I implement IntelliSense / autocomplete UI?

**A:** Wire `lspRequestCompletion(uri, line, col, [trigger])` to a key event (typically the `.` / `->` / `::` triggers, or `Ctrl+Space`). Render the response in your own popup widget (the editor doesn't ship one). Use the `<kui-menu>` built-in or roll your own.

### Q: Can two editors share an LSP client?

**A:** No. Each editor instance owns its `lspManager`. To share a server, run two `clangd` processes (cheap) or build a host-side multiplexer that forwards requests.

### Q: How do I implement vim mode?

**A:** Intercept `onKeyEvent` in your application before the editor sees it (your custom element wrapping `<kui-editor>`). When in normal mode, capture `j` / `k` / `w` / etc. and translate to the editor's movement methods (`moveDown` / `moveUp` / `moveWordRight`). Re-emit chars to the editor only in insert mode.

A bundled vim layer is on the long-term roadmap.

### Q: Can I theme bracket-pair colors?

**A:** Yes - the theme has a `bracketColors[6]` array. Edit `editor_theme.cpp::themeXxx()` and rebuild.

### Q: Does the editor support virtual scrolling for million-line buffers?

**A:** The buffer is a `vector<string>` today (M0-S1). It works comfortably up to ~50 K lines but degrades on the megaline scale due to `std::vector` reallocation costs and lack of an incremental layout cache. **M2+ replaces this with a Piece Tree** for proper million-line support. As of v1.9.0 you should expect best results at < 100 K lines.

### Q: Can I disable IME composition?

**A:** No - IME is always on, controlled by the OS. If your application doesn't need East Asian input, you can ignore the IME state; the editor handles it transparently.

### Q: How do I handle "modified-but-unsaved" UI state?

**A:** Poll `getBufferContentVersion()`. Save a snapshot when you persist to disk; compare on every poll. When they differ, mark the tab dirty.

```lua
local savedV = ed:getBufferContentVersion()
function isDirty() return ed:getBufferContentVersion() ~= savedV end
function markSaved() savedV = ed:getBufferContentVersion() end
```

### Q: Why are some Lua methods undocumented?

**A:** This document covers the stable, supported API. A handful of `*Instr*`, `*Bench*`, `lspInject*` methods are debug / test helpers - they exist but their semantics may change without notice. Don't rely on them in production.

### Q: How do I add a new language?

**A:** Three steps:

1. Vendor the tree-sitter grammar into `extensions/kui_ext_editor/third_party/tree-sitter-{lang}/`.
2. Add the `extern "C" const TSLanguage* tree_sitter_{lang}();` to `editor_treesitter.h` and the lookup branch to `editor_treesitter.cpp::lookupTsLanguage`.
3. Add `highlight.scm` and `folds.scm` queries to `editor_treesitter_queries.cpp`.

Rebuild the DLL. The new name will work in `setLanguage()` and the `language=` attribute.

---

## 33. View Zones & Line Widgets (Inline Diff Primitives)

> Added for Forge feedback **KUI-020** — the two primitives needed for
> Monaco-style inline diff review: phantom rows that show deleted/added
> content in place, plus clickable accept/reject buttons pinned to a line.
> Same feed-data + poll contract as `setGitDiff` / `consumeGutterClick`:
> line anchors are **not** auto-shifted on buffer edits — re-publish after
> a change.

### 33.1 View zones (phantom rows)

```lua
-- Insert non-document display rows AFTER a buffer line.
-- afterLine: 0-based; -1 inserts above the first line.
-- rowsJson : JSON array of strings, one per row.
-- kind     : 0 neutral, 1 deleted (red tint + gutter strip), 2 added (green).
local zoneId = ed:addViewZone(4, '["- local x = compute()", "- use(x)"]', 1)
ed:removeViewZone(zoneId)      -- -> true/false
ed:clearViewZones()
local n = ed:getViewZoneCount()
```

Zones occupy real display rows: every line below shifts down, the scroll
range grows, and `caretRect` / `posToPixel` / `pixelToPos` / click
hit-testing all account for them.  A click inside a zone row resolves to
the buffer line the zone precedes.  `addViewZone` returns `0` on invalid
input (bad line / kind / JSON); validation is atomic — state is never
half-updated.

### 33.2 Line widgets (anchored buttons)

```lua
-- Pin a clickable button row to a buffer line.
-- placement: 0 above, 1 below (each occupies one display row), 2 end-of-line.
local wid = ed:addLineWidget(4, 0,
    '{"buttons":[{"id":"accept","text":"✓ Accept"},{"id":"reject","text":"✗ Reject"}]}')

-- Poll for clicks (e.g. in the same tick loop as consumeGutterClick):
local hit = ed:consumeLineWidgetClick()   -- "widgetId,buttonId" or ""
if hit ~= "" then
    local widgetId, buttonId = hit:match("^(%d+),(.+)$")
    -- apply or discard the hunk, then remove the widget + zones
end

ed:removeLineWidget(wid)
ed:clearLineWidgets()
local m = ed:getLineWidgetCount()
```

The payload is backward compatible.  Add `"variant":"segmented"` for a
single capsule group; every button may add `"tone":"neutral"`,
`"positive"`, or `"danger"`, plus `"disabled":true`:

```lua
local wid = ed:addLineWidget(4, 0, [[{
  "variant":"segmented",
  "buttons":[
    {"id":"accept","text":"✓ Accept","tone":"positive"},
    {"id":"reject","text":"✗ Reject","tone":"danger"},
    {"id":"later","text":"Later","tone":"neutral","disabled":true}
  ]
}]])

-- Optional per-editor token override. Colors are #RRGGBB or #RRGGBBAA;
-- geometry is in CSS px. Missing fields preserve the active theme token.
ed:setLineWidgetTheme([[{
  "height":24,"radius":9,"paddingX":10,"gap":6,
  "background":"#3A60A0C8","foreground":"#E6EBF5FF",
  "border":"#6E7891DC","hover":"#4B73B9EB","pressed":"#2D4E87FF",
  "positive":"#2D875AEB","danger":"#B23E49EB",
  "disabled":"#9196A5B4","focus":"#89B4FAFF"
}]])
local tokensJson = ed:getLineWidgetTheme()
```

Buttons move with scrolling and never move the caret. Pointer hover/pressed,
the last pointer-focused button, Enter/Space activation, disabled suppression,
and one-shot click polling are handled by the self-drawn control. A typical
inline diff hunk is one red zone (old lines) + one green zone (new lines) +
one segmented accept/reject widget row.

Runnable end-to-end demo (two AI hunks, accept/reject wired up):
`examples/native/extension/15_kui_editor_inline_diff_demo/`.

### 33.3 Line decorations & gutter markers (KUI-004 / KUI-023)

Full-row background tints and gutter glyphs — the building blocks for
debugger UIs and the "buffer already contains the new content" side of a
diff review (phantom zones above show what was removed; the real, inserted
lines get a green row via kind 10).

```lua
ed:setLineDecoration(line, kind)   -- 0-based line; kind <= 0 clears
local k = ed:getLineDecoration(line)  -- read-back, 0 = none
ed:clearLineDecoration(line)
ed:clearAllLineDecorations()
```

| kind | Meaning | Visual |
|------|---------|--------|
| `1` | current execution line | amber row tint |
| `2` | error row | red row tint |
| `3` | warning row | orange row tint |
| `10` | **DIFF_ADDED** | **green** row tint (theme `gitAdded` slot) + gutter `+` glyph |
| `11` | **DIFF_REMOVED** | **red** row tint (theme `gitDeleted` slot) + gutter `−` glyph |
| other > 0 | reserved | renders as kind 1 |

Kinds 10/11 are theme-aware (they derive from the active `EditorTheme`'s
git colour slots, so every flavor matches the git gutter). On rows with
both a gutter marker (below) and a diff kind, the marker keeps the slot;
the diff glyph in turn outranks the LSP severity dot. View-zone rows with
kind 1/2 also draw the matching `−`/`+` glyph in their (empty) line-number
column.

```lua
ed:setGutterMarker(line, kind)     -- kind <= 0 clears
local k = ed:getGutterMarker(line)
ed:clearGutterMarker(line) / ed:clearAllGutterMarkers()
local clicked = ed:consumeGutterClick()  -- 0-based line, -1 = none (poll)
local metrics = json.decode(ed:getGutterMetrics())
```

| kind | Meaning | Visual |
|------|---------|--------|
| `1` | breakpoint | red filled circle |
| `2` | conditional breakpoint | amber filled circle |
| `3` | disabled breakpoint | grey circle outline |
| `4` | current-execution marker | blue filled circle |

The gutter has two stable logical-pixel regions: a 16px marker slot on the
left and a right-aligned line-number slot on the right. The marker geometry is
independent of line-number digit count, editor font size, device pixel ratio,
horizontal scrolling, and sticky rows. Clicking either slot is reported by
`consumeGutterClick()` using the same 0-based source line contract.

`getGutterMetrics()` returns read-only viewport geometry for host automation:

```json
{"width":50.04,"markerSlotWidth":16.00,
 "lineNumberStart":24.00,"lineNumberEnd":42.04}
```

All values are logical content-box pixels. `width` is the complete gutter;
`lineNumberStart` / `lineNumberEnd` delimit the number area after its internal
padding. When the gutter is hidden, all four values are zero.

A typical applied-hunk review row set:

```lua
-- removed lines: phantom rows above the edit
ed:addViewZone(afterLine, '["- old line A", "- old line B"]', 1)
-- inserted (real) lines: green row + "+" glyph
for l = firstNew, lastNew do ed:setLineDecoration(l, 10) end
```

---

## 34. See Also

- `extensions.md` - The DLL extension system the editor is built on
- `components.md` - Custom element / virtual component framework
- [`lua.md`](./lua.md) - Lua scripting + DOM API the editor methods plug into
- [`devtools.md`](./devtools.md) - Inspect editor state via DevTools (Components panel shows `<kui-editor>`)
- `api_reference.md` - SDK API reference index
- [`features.md`](./features.md) - HTML / CSS feature matrix the editor sits inside
- `tutorial.md` - General KUI walkthrough
- `faq.md` - Cross-cutting FAQs
- `extensions/kui_ext_editor/` - The editor's source code
- `extensions/kui_ext_editor/src/editor_state.h` - Per-element state (~755 lines, full field list)
- `extensions/kui_ext_editor/src/editor_lua_api.cpp` - All 200+ Lua method implementations
- `extensions/kui_ext_editor/src/editor_events.cpp` - Keyboard / mouse / wheel / IME event handlers
- `extensions/kui_ext_editor/docs/` - Internal design docs (FR / NFR specs, milestone breakdowns)
- [`tree-sitter`](https://tree-sitter.github.io/) - Parser library used for syntax highlighting
- [`PCRE2`](https://www.pcre.org/) - Regex engine used for find/replace
- [Catppuccin theme](https://github.com/catppuccin) - Source of the Mocha / Latte palettes
- [LSP specification](https://microsoft.github.io/language-server-protocol/) - Protocol used by the LSP integration

---

## 自动补全人体工学 API（KUI-036，v3.14.0）

为商业级自动补全（LSP 打字触发 + AI ghost text 行内补全）补齐的 5 项增强 API。全部向后兼容；事件相关的需求 3/4 依赖宿主 API ≥ v10（KUI 引擎 3.14.0+），在更旧宿主上自动降级为静默 no-op，而纯查询 API（需求 1/2/5）不受宿主版本限制。

### 文本访问

| 方法 | 签名 | 说明 |
|------|------|------|
| `getLineText(line)` | `(int) -> string` | 取第 `line` 行（0-indexed）文本（不含换行），越界返回 `""`。复杂度 O(行长)。 |
| `getTextRange(sl,sc,el,ec)` | `(int,int,int,int) -> string` | 取 `(sl,sc)`→`(el,ec)` 范围文本。字节列（UTF-8）；多行以 `\n` 连接，与 `getValue()` 一致。参数自动 clamp 到 buffer 范围，反向范围自动交换。 |
| `getCaretByteOffset()` | `() -> int` | 主光标在整 buffer（`getValue()` 单 `\n` 连接、无尾换行）中的绝对 UTF-8 字节偏移。复杂度 O(caretLine)。 |

```lua
-- 取光标前缀（FIM prefix）
local off = editor:getCaretByteOffset()
local prefix = editor:getTextRange(0, 0, editor:getCaretLine(), editor:getCaretCol())
-- 仅取当前行做触发判断
local lineText = editor:getLineText(editor:getCaretLine())
```

### 内容变化事件（`input`）

`<kui-editor>` 在每次 buffer 变化时派发标准 DOM `input` 事件，替代 `getBufferContentVersion()` 轮询（即时 + 精确，无需回读光标前字符）。

```lua
editor:addEventListener("input", function(e)
    -- e.data      = 本次输入文本（删除时为 ""）
    -- e.inputType = 见下表
end)
```

`e.inputType` 取值（对齐 W3C InputEvent）：

| inputType | 触发场景 |
|-----------|----------|
| `insertText` | 普通字符输入 / Tab / 选区替换 / 自动配对 / 智能缩进 |
| `insertLineBreak` | 回车换行 |
| `insertFromPaste` | 粘贴 |
| `deleteContentBackward` | Backspace |
| `deleteContentForward` | Delete |
| `deleteByCut` | 剪切 |
| `insertReplacementText` | 行级操作（注释切换 / 移动 / 排序等） |

> 注：undo / redo 重放不派发 `input`（与轮询版 `getBufferContentVersion` 行为一致，仍会自增）。

### Ghost text 接受 / 取消事件

ghost text 状态迁移时派发，解锁 AI 链式补全 + 接受率埋点 + 取消原因感知。两类事件均复用 `InputEvent` 承载，宿主统一读 `e.data` / `e.inputType`。

```lua
editor:addEventListener("ghostaccept", function(e)
    -- e.data      = 被接受的文本
    -- e.inputType = "full"（整段：acceptGhostText / Tab）| "partial"（逐词：acceptGhostWord）
end)

editor:addEventListener("ghostdismiss", function(e)
    -- e.inputType = "escape"（Esc 主动取消）| "typing"（打字）| "caretmove"（移动光标 / 点击 / setCaret）
end)
```

> 接受 ghost 时会同时派发一个 `input` 事件（buffer 确实发生了插入）与一个 `ghostaccept` 事件；宿主可据 `ghostaccept` 区分「ghost 接受」与普通输入，避免在接受后重复触发补全。

### LSP server capabilities 透传

| 方法 | 签名 | 说明 |
|------|------|------|
| `lspServerCapabilitiesJson()` | `() -> string` | 返回已 attach 的 LSP server `initialize` 响应中的 `capabilities` JSON 字符串；未连接 / 未初始化时返回 `"{}"`。 |

```lua
local caps = require("json").decode(editor:lspServerCapabilitiesJson())
local triggers = (caps.completionProvider or {}).triggerCharacters or { ".", "->", "::" }
```

据权威 `completionProvider.triggerCharacters` 精确触发，多语言通用；亦可读取 `resolveProvider`（补全项详情懒加载）、`signatureHelpProvider.triggerCharacters`（签名帮助触发）等。
