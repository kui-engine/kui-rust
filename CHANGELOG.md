# KUI Rust SDK Changelog

## [Unreleased]

### Changed

- canonical 示例增至 225 个（211 个与 C++ 原版一一对应 + 14 个 API 巡览示例），`bin/` 共 233 个程序；`build.ps1` / `build.sh`
  的数量门改读包内 `examples/canonical_manifest.json`，不再写死。预编译示例包的 `bin/` 另带 `kui_script_example` 用的 `script_html/`。
- `kui-sys` 在没有 `KUI_C_SDK` / `KUI_C_LIB_DIR` 时依次找 SDK 包布局 `<包根>/native`、对外仓库（`git clone kui-rust`）布局
  `<仓库根>/runtime/<平台>`、KUI 仓库内开发布局，`git clone` 后 `cargo build` 不用设环境变量；都找不到时报错并列出试过的目录。
  两个环境变量仍然优先、行为不变（给了 `KUI_C_SDK` 就只在它下面找）。`build.ps1` / `build.sh` 不给参数时同样依次找这两种布局。
  Linux / macOS：kui-sys 选中的运行库目录在本仓库 `runtime/` 下时，`kui` crate 的示例与测试再加一条指向它的运行时搜索路径，
  `cargo run --example` / `cargo test` 不用设 `LD_LIBRARY_PATH`；SDK 包与 KUI 仓库内照旧只有 `$ORIGIN`（预编译程序不带构建机路径）。

### Added

- 宿主集成：`WindowProps` 的 `sizeable` / `hidden` / `frame_radius`、`Window::set_enabled` / `is_enabled` / `process_frame` / `set_devtools_token_auth`、`Element::start_momentum_scroll` / `scroll_into_view`、`ScriptEngine::tick_timers`、`Event::wheel_delta`、`Element::add_capture_event_listener`（捕获阶段监听）、`Document::set_base_path` / `base_path` / `set_render_device_pixel_ratio`、`console_set_*`、`stack_trace_install` / `crash_reporter_install`、`disable_ime`。
- 补齐缺口：`Observable` / `Observer` 实现 `Clone`（对应 retain）、`Router::current_path`、`TreeView::update_node`（`TreeNodeUpdate`，`None` 保持原值）、`as_raw()`（`Document` / `Element` / `Observable` / `Observer`，给 `kui::sys` 的低层调用用）。
- `Event::related_target`：鼠标与焦点事件的 `relatedTarget`；`<kui-context-menu>` 的 `menu-open` 里是被右键的元素。
- `Popups::set_default_context_menu_enabled` / `default_context_menu_enabled`：引擎自带的默认右键菜单 3.61.8 起默认
  不显示，用它开启（页面也可以写 `<html context-menu>`）。
- Linux x64 / macOS arm64：`kui-sys` 按目标平台链接 `kui_c.lib` / `libkui_c.so` / `libkui_c.dylib`（`KUI_C_LIB_DIR`、
  `KUI_C_SDK` 可覆盖）；`kui` crate 的示例与测试带 `$ORIGIN`（macOS `@executable_path`）运行时搜索路径。包内 `build.sh`
  与 `build.ps1` 同一组步骤；只有 Release。`HTTPClient` 在这两个平台上由内核的 libcurl（Linux，运行期加载发行版的
  `libcurl.so.4`）/ NSURLSession（macOS）实现，语义与 Windows 相同（`T-DEBT-POSIX-HTTPCLIENT-NOT-IMPLEMENTED`）。
- `kui::post_to_main_thread`：从任何线程把 `Send + 'static` 闭包交给 UI 线程执行；
  平台未初始化或已开始关闭时返回错误，关闭时还在排队的闭包在 UI 线程上 drop、不执行。
- `HTTPClient::fetch_async_on_main_thread`：回调在 UI 线程执行，闭包不要求 `Send`
  （可以捕获 `Rc` 等 UI 对象）。这是安全接口里的第一批回调。
- 安全接口的回调：`Element` / `Window` 的 `add_event_listener`、`remove_event_listener(s)`、
  `Element::dispatch_event`；`Event`（类型、种类、目标、标志、鼠标坐标与按键、键盘、拖入文件，
  只在回调内有效）；`Window::on_resize`；`Platform::set_timeout` / `set_interval` /
  `clear_timer` / `request_idle`；`ScriptEngine::set_error_handler` / `unbind_document`；
  `HTTPClient::fetch_async`（工作线程回调，闭包须 `Send`）。闭包不要求 `Send`，保存在
  UI 线程的登记表里，回调中移除自己也安全。
- 安全接口补齐到冻结清单内的全部导出：`Document` 的 `root` / `body` / `head` /
  `query_selector_all` / `load_html_file` / `clear_stylesheets` / 自定义元素
  （`CustomElementDefinition`）/ 扩展状态与性能统计；`Element` 的树遍历、子树查询、`id`、
  `class_name`、classList、`has_attribute` / `remove_attribute`、innerHTML、样式文本与属性名、
  `!important`、client / offset / scroll 几何、滚动、focus / blur、`position_relative_to`、
  `remove` / `remove_child`、媒体读写与 Lottie；`Window` 的尺寸位置、可见 / 最小化 / 最大化、
  不透明度、总在最前、DPR、无框可调尺寸、圆角、DevTools 端口与地址、原生句柄；`Platform`
  的显示器、剪贴板查询、F12 开关与文件对话框；`Document` 上的 DevTools 鼠标 / 键盘注入与
  盒模型。`Document` 与 `Element` 实现 `Clone`（retain）。
- 常用窗口、DOM 与平台操作：`Window::focus` / `has_focus` / `set_cursor`（`Cursor`）/
  `set_cursor_visible` / `set_fullscreen` / `is_fullscreen` / `set_frameless` /
  `set_frameless_shadow` / `frame_state` / `set_frame_radii` / `frame_radii` /
  `set_ime_candidate_pos`；`Element::insert_before` / `replace_child` / `clone_node` / `matches` /
  `closest` / `outer_html` / `insert_adjacent_html` / `append_text` / `has_focus` / 指针捕获；
  `Document::get_elements_by_class_name` / `get_elements_by_tag_name` / `element_from_point`；
  `Platform::message_box` / `open_url` / `now` / `monitor_from_point` / `monitor_from_window`。
- 桌面集成：`Window::set_menu_bar` / `clear_menu_bar` / `set_menu_item_checked` /
  `set_menu_item_enabled`（`Menu` / `MenuItem`）；托盘 `create_tray` / `destroy_tray` /
  `set_tray_visible` / `set_tray_tooltip` / `set_tray_icon` / `set_tray_context_menu` /
  `popup_tray_context_menu` / `show_tray_balloon` / `on_tray_event` / `simulate_tray_event`；
  `bind_shortcut` / `unbind_shortcut` / `unbind_all_shortcuts` / `set_shortcuts_enabled`；
  `Document::show_toast` / `dismiss_toast` / `dismiss_all_toasts` / `toast_count`
  （`ToastOptions`）；`Window::save_screenshot`。闭包不要求 `Send`，替换、移除或窗口销毁时释放。
- 扩展与 DevTools 生命周期：`Document::load_extension` / `unload_extension` / `reload_extension` /
  `unload_all_extensions` / `watch_extensions` / `extension_state`，`Window::stop_devtools` /
  `devtools_state`。
- 拖放数据：`Event::set_drag_data` / `drag_data`（未设置为 `None`）/ `clear_drag_data`（`None` 清全部）/
  `drag_types` / `set_drop_effect` / `drop_effect` / `set_effect_allowed` / `effect_allowed`
  （`DropEffect`、`EffectAllowed`）。
- 脚本引擎控制：`ScriptEngine::eval_value` / `eval_file` / `call_function`（返回 `ScriptOutput`，含第一个
  返回值）、`register_function` / `create_function`（闭包返回 `Err(文字)` 即 Lua 错误；同一个闭包不能在
  自己执行期间被再次调用）、`set_global` / `global`、`set_print_handler` / `clear_print_handler`、
  `set_memory_limit` / `set_instruction_limit` / `resource_usage`；`ScriptValue`、`ScriptFunction`
  （克隆共享句柄，drop 释放）、`ScriptResources`。
- 自定义元素自绘、tick 与输入：`Document::define_custom_element_with` 与 `CustomElementHooks`（paint / tick /
  needs_tick / key / mouse / wheel / ime；原有 `CustomElementDefinition` 不变，字面量构造不受影响）；
  `PaintContext`、`TextStyle` / `TextMetrics` / `CornerRadii` / `DecorationStyle`、输入事件类型与
  `input_modifiers`；`Element::measure_text` / `request_repaint` / `invalidate_rect`、
  `Document::dispatch_ime_event`。
- 组件：`Document::register_builtin_components`；`ThemeManager`；`VirtualList`（`VirtualListConfig`，`render_item`
  返回的 `Element` 由 KUI 接管）、`ListView`（`ListViewConfig` / `ListViewItem` / `SelectionMode`）、`TreeView`
  （`TreeNode` / `TreeViewConfig`）、`ScrollUnit` / `VirtualListState`。都在 drop 时释放。
- 文档样式配置：`Document::set_style_profile` / `style_profile`（`StyleProfile`）。
- canonical 示例 runner 与 C++ 原版对齐：托盘（以前完全没有）、多窗口一起关闭、html_loader 的浏览器默认值 / 内置
  组件 / 原生弹出层 / 三个列表；窗口选项照原版，补上无边框阴影与拖边缩放。
- 媒体与 Lottie 播放控制：`Element::load_media` / `set_media_loop` / `set_media_autoplay` /
  `media_playback_flags`，`Element::stop_lottie` / `seek_lottie` / `set_lottie_speed` / `set_lottie_loop` /
  `lottie_state`（`LottieState`、`LottieLoadState`）。
- 弹出层 / 选择器 / 音频：`Document::popups()`（`Popups`）、`Element::open_input_picker` 与
  `Document::input_picker()`（`InputPicker`）、`Document::audio()`（`Audio`、`AudioPlayOptions`、
  `SoundInfo`）。右键菜单的闭包在菜单关闭后释放。
- `Element::computed_style_value`：按 CSS 属性名读取计算后的值（等于
  `getComputedStyle(el).getPropertyValue(name)`）。
- 应用框架：`Document::router()`（`Router::add_route` / `remove_route` / `clear` / `has_route` /
  `route_count` / `register_view` / `unregister_view` / `push` / `replace` / `go` / `back` / `forward` /
  `current` / `param` / `history` / `add_guard` / `remove_guard` / `on_navigated` / `remove_navigated`，
  `RouteInfo`）；`Observable`（`new` / `set` / `get` / `subscribe` / `unsubscribe` / `bind` / `unbind` /
  `unbind_all`，drop 时释放）与 `Document::flush_bindings`；`Element::animation_state`、
  `Document::animation_counts`；`Document::create_mutation_observer` / `create_resize_observer` /
  `create_intersection_observer`（`Observer::observe_mutations` / `observe` / `unobserve` /
  `disconnect`，drop 时释放）。KUI 通知释放的闭包推迟到下一个安全点才 drop（通知可能在文档析构中途到来）。
- `WindowProps` 新增 `frameless_shadow`、`maximized`、`min_width`、`min_height`、`max_width`、
  `max_height`。用结构体字面量构造时请保留 `..WindowProps::default()`（文档与示例一直这样写）。
- 需要 3.61.8 及以上的 `kui_c.dll`（新增两个导出）。
- `Element::value / set_value / checked / set_checked / selected_index / set_selected_index`：读写表单控件的当前状态（对应 C ABI 新增的 6 个导出）。

### Fixed

- Windows：包内 `build.ps1` 在设了 `CARGO_TARGET_DIR` 时找不到编译好的示例（写死 `<sdk>\target`）；现在与 `build.sh` 一样从
  `CARGO_TARGET_DIR`（未设时 `<sdk>\target`）收集。3.61.8 发版包级验收拦下。
- 监听闭包捕获了 `Document` / `Element` 的克隆时，进程退出以「thread local panicked on drop」中止（线程局部的回调登记表
  销毁时 drop 闭包，闭包拥有的文档析构又回到这张表）。注销 / 释放路径改用 `try_with`。
- `ScriptEngine::eval` 以前用 4 KB 缓冲接 Lua 错误文本，更长的错误让 `eval` 返回 `Err`
  （`KUI_ERR_BUFFER_TOO_SMALL`）而不是结果；现在读完整的错误文本。
- 安全接口传空字符串（`set_text_content("")`、属性值为空等）会让 C ABI 读到空切片的悬空
  指针（C ABI 约定长度 0 时按 NUL 结尾读取），进程访问冲突；现在空字符串改传真正的空 C 字符串。
- `Element::invoke_extension` 以前不看调用状态：方法不存在、参数个数不对、扩展回调失败时
  返回空结果；现在返回错误，消息里带扩展给出的错误文本。
- `build.ps1 -Configuration Debug` 的输出改为「Debug 核心（`kui_c.dll`、`kui.dll`）+ Release 扩展运行时」，与 C++ / C SDK 的部署约定相同（以前 Debug 输出里放的是 Release 核心）。Debug 默认输出改为 `bin\debug`，不再覆盖 `bin/` 里的标准 Release 产物。仓库内开发时，运行时目录与 `kui-sys/build.rs` 选的导入库同出一处（`sdk/c/build/<cfg>`），不再取上一次发版同步回来的旧 `sdk/c/bin`——C ABI 新增导出后，旧 DLL 会让测试起不来。

### Changed

- 随包的冻结头 `kui-sys/include/kui_c.h` 重新与权威 `sdk/c/include/kui_c.h` 同步：此前一直停在
  3.54.0（只差版本宏与几何取整注释，函数签名未变，绑定代码不变）。发版预检、打包与
  `scripts/check_sdk_versions.py` 现在都会校验它。
- 211 个 canonical 示例里有 47 个的窗口尺寸改为与 C++ 原版一致：生成器以前对写在共享头里、
  或用变量给出的尺寸一律退回 960×700（例如 3D 系列应为 1100×760）。manifest 同步更新。

### 更正

- 3.1.0 的发布说明写「正式包……进行解包重建和全量启动」，实际包验证只做了解包重建与
  数量检查，从未启动过这 219 个程序；全量启动门另行补上（仓库 `TODO.md` 的
  `T-DEBT-SDK-GORUST-NO-LAUNCH-GATE`）。

## [3.1.0] - 2026-08-26

- 首个 Windows x64 Full Profile GA 版本。
- 新增独立 `kui-sys` crate，机械生成并检验全部 195 个 C ABI 声明，consumer
  无需 bindgen/libclang。
- 新增安全 `kui` 门面，以 RAII、`Result` 和 `!Send + !Sync` 封装核心开发路径；
  公开 `kui::sys` 作为完整 ABI escape hatch。
- 覆盖 Window、Document/DOM/style、Lua、HTTP、media、PDF 与 extension/3D。
- 增加自托管 `build.ps1`、ABI layout test、Full Profile smoke test、8 个 quickstart
  和与 C++ 产品清单一一对应的 211 个 canonical examples；正式包交付全部 219 个
  标准预编译 EXE、页面、素材与 Full Profile runtime，并进行解包重建和全量启动。

## [3.1.0-preview] - 2026-05-23

- 发布 API 设计草案；没有连接可执行 C ABI，不能用于开发或发布。
