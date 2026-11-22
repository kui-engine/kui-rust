# KUI CSS 特性支持矩阵入口

完整的 CSS 支持地图（可浏览、可搜索的清单）、它的数据文件和生成脚本由开发团队内部维护，**没有随公开的 SDK 包与
本仓库提供**；本页保留其中与范围、实现状态有关的部分。

2026-11-01 重新从当前源码全文发现属性入口，并沿解析、级联、样式写入、布局、绘制、
动画和交互消费链审计，交叉检索 examples 与测试。旧版 2026-02 的汇总支持率、
“注册/解析即支持”结论已从活跃入口移除；历史内容可通过 Git 追溯。
本页不再重复维护另一张容易漂移的逐属性表。

## 读法

- **已实现**：列出的语法有实际消费路径；不表示整个标准模块、所有取值或所有组合完全兼容。
- **部分实现**：必须同时阅读值域和限制，包含简写、单位、列表、语义近似和特定组合缺口。
- **条件支持**：依赖平台、后端、特定控件或 PDF/top-layer 等入口。
- **未生效 / 不支持**：也包括仅注册、仅保存、无消费者与兼容 no-op。

源码、examples 和测试链接只是可追溯入口。本次是静态源码审计，没有执行全量 CSS/WPT/像素
验收，不把历史报告或旧 EXE 的通过结果当作新鲜证据，不计算浏览器兼容率。

## 实现状态与 Scope 分开

`Scope` 是 ADR/WPT 的纳入范围，不是代码“能否执行”的布尔值。原有范围决策仍见
[ADR-001](adr/ADR-001-out-of-scope-features.md)，本次没有调整这些决策或正式 WPT 基线。

| 能力 | Scope | 备注 |
|---|---|---|
| 多栏 | OUT_OF_SCOPE | 现有 column 布局、spanner、碎片绘制路径，不能再描述成只有解析；复杂组合有限，见 HTML 各属性。 |
| Subgrid | OUT_OF_SCOPE | Grid 中有裸 subgrid 的父轨道消费；命名线/intrinsic 等有限，不能从范围标记推断整条运行时禁用。 |
| CSS Shapes | OUT_OF_SCOPE | 当前通用属性与布局没有 shape-outside/shape-margin/shape-image-threshold 消费，不因示例/上游语料出现而视为实现。 |
| SVG 引用 filter | OUT_OF_SCOPE | CSS 内置滤镜与 SVG filter url 引用分开；filter:url(#...) 没有通用滤镜消费。 |
| 打印 / 分页 | PARTIAL | Document::printToPDF() 有真实页几何、页规则、基础 fragmentation；尚无本轮逐页 print reftest 验收。 |
| Container style query | DEFERRED_TO_3.0 | 基础数值容器查询有有限实现，style()、scroll-state() 和完整条件语法不支持。 |
| @scope | DEFERRED_TO_3.0 | parseAtRule 无对应处理，跳过规则；不能把 :scope 伪类当成 @scope 规则支持。 |

`include/kui/config.h.in` 明确说明 `KUI_DISABLE_MULTICOL`、`KUI_DISABLE_SUBGRID`
等是声明性范围标记；当前生产源码没有对应 `#if` 门控。实现存在与完整一致性是两件事，
HTML 对实际可达路径和限制逐项说明。

## 更新方法（开发团队内部流程；其中的脚本与数据文件不在公开包和本仓库里）

```powershell
python scripts/generate_css_support.py --check
python scripts/generate_css_support.py
# 源码/示例变化后，先复核数据和证据，再更新扫描快照：
python scripts/generate_css_support.py --refresh-inventory
```

检查器验证注册名、逻辑映射/简写、声明展开、all 级联入口及 KUI 组件主题变量没有漏项，
验证条目唯一、证据路径/行界有效、全文内容指纹未变、HTML 与 JSON 一致。
指纹更新只记录发现范围，绝不自动将新名字判为支持。新增语义仍须沿真实消费路径审计。

本次依据、统计、验证与边界见
工作日志。
