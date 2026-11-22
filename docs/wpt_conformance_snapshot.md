# KUI WPT 一致性快照

- KUI 版本：`3.61.8`
- 平台：`windows`
- 基线 schema：`2`
- 基线采集自提交：`4dd58b48459879b7be339b56b1ad35da7c8713bd`
- 本文件生成时间（UTC）：`2026-11-22T15:44:48Z`

## 这份快照是什么

它由 `tests/wpt/baselines/windows.json` 直接折算而来 —— 那是发布质量门**逐项精确比对**的同一份文件。发布门的判据是：全部 WPT runner 正常退出、`[CONFORMANCE FAIL]` 为 0，且逐项状态与本基线**完全一致**（key、状态、PASS 集合三者都不允许漂移）。所以下表的 PASS 数就是本版实际通过数，不是历史宣称值。

状态词含义：

- **PASS**：runner 判定与规范期望一致。
- **EXCLUSION_SKIP**：`tests/wpt/conformance_skiplist.txt` 明列的范围外用例，依据是 `docs/adr/ADR-001-out-of-scope-features.md` 与范围外特性（multicol / CSS Shapes / subgrid；`KUI_DISABLE_*` 只是声明性标记，不改变行为）。
- **FEATURE_SKIP**：runner 运行时特性检测判定该用例依赖的能力未编译进当前构建，因此不产生结论。

**为什么没有 FAIL 这一列**（consumer 审查单 `ASK-KUI-3.57.0-R2` G6 问到）：发布门的硬判据是 `[CONFORMANCE FAIL]` 必须为 0，所以只要基线是绿的，这张表里就不会出现 FAIL。**不要据此读成「没有不通过的用例」**——跑不过的用例在进入基线之前，要么被 `conformance_skiplist.txt` 显式排除（EXCLUSION_SKIP），要么被 runner 的特性检测挡在执行之前（FEATURE_SKIP）。

**因此 SKIP 的诚实读法是**：FEATURE_SKIP ≈「该用例依赖的能力在本构建里不可用，所以没跑」——它**确实指向一处能力缺口**，只是不必然是「缺陷」（可能是 `known_issues.md` 里已登记、已排期的未实现项）。此前这里写的是「SKIP 不等于 KUI 不支持该特性」，那个说法过宽，已订正。

能力判定请以 `docs/css_feature_matrix.md`（声称支持什么）与本表（实际通过什么）**交叉**为准。特别提醒：`include/kui/config.h` 里那三个 `KUI_DISABLE_*` 宏是**声明性的、不改变引擎行为**（`T-DEBT-SCOPE-FLAGS-ARE-DECLARATIVE-ONLY`），所以 multicol 这类「声称禁用」的套件里出现真实 PASS 是预期的，不是统计错误。

注意 PASS 率的读法：分母含 SKIP 时会低估，不含时会高估。两个数都给在「总计」里，请按你的问题选分母，不要把任一侧当成「浏览器一致性得分」。

## 总计

- 套件数：**36**
- 用例数：**9515**
- EXCLUSION_SKIP：**297**
- FEATURE_SKIP：**2823**
- PASS：**6395**

## 逐套件

| 套件 | 用例数 | EXCLUSION_SKIP | FEATURE_SKIP | PASS |
|---|---:|---:|---:|---:|
| `css_CSS2` | 287 | 16 | 65 | 206 |
| `css_compositing` | 48 | 1 | 41 | 6 |
| `css_css-align` | 58 | 0 | 14 | 44 |
| `css_css-backgrounds` | 528 | 35 | 29 | 464 |
| `css_css-borders` | 21 | 1 | 14 | 6 |
| `css_css-box` | 46 | 9 | 6 | 31 |
| `css_css-cascade` | 43 | 1 | 19 | 23 |
| `css_css-color` | 158 | 3 | 46 | 109 |
| `css_css-content` | 23 | 0 | 4 | 19 |
| `css_css-display` | 72 | 0 | 32 | 40 |
| `css_css-flexbox` | 824 | 7 | 152 | 665 |
| `css_css-fonts` | 242 | 4 | 183 | 55 |
| `css_css-gaps` | 215 | 9 | 70 | 136 |
| `css_css-grid` | 657 | 38 | 218 | 401 |
| `css_css-images` | 225 | 15 | 75 | 135 |
| `css_css-inline` | 145 | 0 | 133 | 12 |
| `css_css-lists` | 136 | 15 | 23 | 98 |
| `css_css-masking` | 196 | 13 | 101 | 82 |
| `css_css-multicol` | 318 | 5 | 90 | 223 |
| `css_css-nesting` | 8 | 0 | 1 | 7 |
| `css_css-overflow` | 417 | 57 | 156 | 204 |
| `css_css-position` | 199 | 0 | 82 | 117 |
| `css_css-pseudo` | 208 | 0 | 153 | 55 |
| `css_css-shadow` | 66 | 1 | 64 | 1 |
| `css_css-sizing` | 421 | 5 | 79 | 337 |
| `css_css-tables` | 114 | 2 | 17 | 95 |
| `css_css-text` | 1146 | 17 | 267 | 862 |
| `css_css-text-decor` | 248 | 3 | 173 | 72 |
| `css_css-transforms` | 371 | 15 | 30 | 326 |
| `css_css-ui` | 950 | 8 | 23 | 919 |
| `css_css-values` | 173 | 3 | 71 | 99 |
| `css_css-variables` | 177 | 0 | 4 | 173 |
| `css_css-writing-modes` | 305 | 0 | 289 | 16 |
| `css_filter-effects` | 139 | 3 | 33 | 103 |
| `css_selectors` | 185 | 1 | 40 | 144 |
| `html_rendering` | 146 | 10 | 26 | 110 |

## 怎么复核

KUI 侧：`python all.py --run wpt-test`（编译全部 runner 并解析 `[CONFORMANCE PASS|FAIL|SKIP]` 行），证据落在 `build-test-report/wpt/run-latest/`。

consumer 侧：本快照是只读证据。WPT runner 依赖仓内构建与语料，不随 SDK 交付；如果需要可运行的 consumer 侧子集，按 `feedback/README.md` 的流程提需求。
