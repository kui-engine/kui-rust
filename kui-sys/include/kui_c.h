/* =========================================================================
 * kui_c.h - KUI C ABI Public Header
 *
 * Why this exists: the C++ SDK exposes STL types across the DLL boundary, so a
 * consumer must match the CRT flavour of the prebuilt binaries. This C ABI has
 * no CRT-matching constraint. The Windows GA matrix covers MSVC and clang-cl
 * with /MD, /MDd and /MTd consumers. Other compilers/platforms are deferred.
 * It is also the intended foundation for Go (cgo) and Rust (FFI) bindings.
 *
 * Status: Windows x64 Full Profile GA. Every KUI_C_API declaration in this
 * header (listed in sdk/c/abi/kui_c_3.exports) is implemented, exported and
 * covered by the product-major-3 ABI
 * freeze gate. The supported surface includes networking, Lua,
 * extension/media, Canvas/WebGL and the standalone 3D extension protocol.
 * Internal C++ implementation types and future native-only APIs are not part
 * of this compatibility claim.
 *
 * Design principles:
 *   - Opaque handles. No exposed struct layouts.
 *   - Status codes via KuiStatus. No exceptions cross the C ABI.
 *   - UTF-8 strings everywhere, with explicit length.
 *   - Explicit ownership, one rule with no exceptions: EVERY function that
 *     hands back a handle gives you a reference you must release. That applies
 *     to lookups (kui_document_get_element_by_id) exactly as it does to
 *     constructors (kui_document_create). See "Ownership" below.
 *   - Stable across product MAJOR version. The product-major export baseline is
 *     sdk/c/abi/kui_c_3.exports; existing functions, enum values and public
 *     structure layouts are immutable throughout 3.x.y.
 *
 * String conventions:
 *   - INPUT strings are (const char* ptr, size_t len). len == 0 means "use
 *     strlen(ptr)". NULL is accepted only where the individual function says
 *     so; required names, selectors, paths and source text reject NULL.
 *   - OUTPUT strings use the two-call pattern:
 *         kui_x_get_y(h, NULL, 0, &need);        // ask for the length
 *         buf = malloc(need + 1);                 // your allocator, your free
 *         kui_x_get_y(h, buf, need + 1, &need);   // fill it
 *     out_len always receives the byte length excluding the NUL terminator.
 *     A NUL is always written when buf_capacity > 0. If the buffer is too
 *     small the call returns KUI_ERR_BUFFER_TOO_SMALL, writes a truncated
 *     NUL-terminated string, and sets out_len to the length actually required.
 *     Nothing is ever allocated on your behalf, so nothing crosses the CRT
 *     heap boundary -- which is the whole point of this API.
 *
 * Ownership:
 *   Handles are reference-counted. A handle stays valid until you release it,
 *   even if the object leaves the DOM tree in the meantime -- so a KuiElement*
 *   never dangles the way a raw C++ Element* would. Releasing a handle does not
 *   destroy the underlying object unless yours was the last reference.
 *
 * Threading:
 *   Same rule as the C++ API: DOM mutation, layout and paint are main-thread
 *   only. Handle retain/release is atomic and safe from any thread.
 *
 * License: MIT (see LICENSE in the SDK package)
 * ========================================================================= */
#ifndef KUI_C_H_INCLUDED
#define KUI_C_H_INCLUDED

#include <stddef.h>
#include <stdint.h>

/* ---------------------------------------------------------------------------
 * Export macro
 *
 * Define KUI_C_STATIC when linking a static build of the C layer.
 * KUI_C_BUILDING_LIBRARY is defined by the kui_c build itself.
 * ------------------------------------------------------------------------- */
#if defined(KUI_C_STATIC)
    #define KUI_C_API
#elif defined(_WIN32)
    #if defined(KUI_C_BUILDING_LIBRARY)
        #define KUI_C_API __declspec(dllexport)
    #else
        #define KUI_C_API __declspec(dllimport)
    #endif
#elif defined(KUI_C_BUILDING_LIBRARY)
    #define KUI_C_API __attribute__((visibility("default")))
#else
    #define KUI_C_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* ---------------------------------------------------------------------------
 * Versioning
 * ------------------------------------------------------------------------- */
#define KUI_C_API_VERSION_MAJOR 3
#define KUI_C_API_VERSION_MINOR 61
#define KUI_C_API_VERSION_PATCH 8

/* Return library version (runtime, possibly different from compile-time macros). */
typedef struct {
    int major;
    int minor;
    int patch;
} KuiVersion;

/* [IMPLEMENTED] Runtime version of the loaded library. */
KUI_C_API KuiVersion kui_get_version(void);

/* ---------------------------------------------------------------------------
 * Status codes
 * ------------------------------------------------------------------------- */
typedef enum KuiStatus {
    KUI_OK                     = 0,
    KUI_ERR_INVALID_ARG        = 1,
    KUI_ERR_OUT_OF_MEMORY      = 2,
    KUI_ERR_NOT_FOUND          = 3,
    KUI_ERR_NOT_SUPPORTED      = 4,
    KUI_ERR_PARSE              = 5,
    KUI_ERR_IO                 = 6,
    KUI_ERR_PLUGIN_MISSING     = 7,
    /* An output buffer was too small. The call still wrote a truncated,
     * NUL-terminated string and set out_len to the length required. */
    KUI_ERR_BUFFER_TOO_SMALL   = 8,
    /* A C++ exception was caught at the ABI boundary. Exceptions never
     * propagate out of these functions. */
    KUI_ERR_INTERNAL           = 99
} KuiStatus;

/* [IMPLEMENTED] Static, never-NULL description. Does not allocate. */
KUI_C_API const char* kui_status_message(KuiStatus s);

/* ---------------------------------------------------------------------------
 * Opaque handle types
 * ------------------------------------------------------------------------- */
typedef struct KuiWindow      KuiWindow;
typedef struct KuiDocument    KuiDocument;
typedef struct KuiElement     KuiElement;
typedef struct KuiEvent       KuiEvent;

/* ---------------------------------------------------------------------------
 * 1. Platform: process lifecycle, event loop, timers            [IMPLEMENTED]
 *
 * There is no handle here, and no "application object". The draft declared a
 * KuiApplication* with kui_application_create/release, but KUI has no
 * Application class at all -- the corresponding C++ API is `kui::Platform`,
 * which is entirely static. Inventing a handle for it would have meant
 * fabricating an object the library does not have.
 *
 * Call order for a real GUI program:
 *     kui_platform_init();
 *     kui_window_create(&props, &win);
 *     kui_window_get_document(win, &doc);   // the window owns its document
 *     kui_document_load_html(doc, html, 0);
 *     kui_window_show(win);
 *     kui_platform_run(&exit_code);         // blocks until quit
 *     ... release handles ...
 *     kui_platform_shutdown();
 * ------------------------------------------------------------------------- */
KUI_C_API KuiStatus kui_platform_init(void);
KUI_C_API KuiStatus kui_platform_shutdown(void);

/* Runs the event loop until kui_platform_quit is called. BLOCKS. */
KUI_C_API KuiStatus kui_platform_run(int* out_exit_code);
KUI_C_API KuiStatus kui_platform_quit(int exit_code);

/* Pump whatever is pending and return immediately -- for embedding KUI in a
 * host that owns its own loop. */
KUI_C_API KuiStatus kui_platform_poll_events(void);
/* Block until an event arrives or timeout_seconds elapses (negative = forever). */
KUI_C_API KuiStatus kui_platform_wait_events(double timeout_seconds);

typedef void (*KuiTaskFn)(void* user_data);
typedef uint64_t KuiTimerId;

/* Timers and idle callbacks run on the main thread from inside the event loop,
 * so they only fire while kui_platform_run / poll_events is active. Like every
 * other function in this header except kui_platform_post_to_main_thread, they
 * must be called on the main thread (the one that called kui_platform_init);
 * from any other thread, hand the work over with kui_platform_post_to_main_thread.
 *
 * user_data is passed through untouched and is never freed by KUI -- keep it
 * alive at least until the timer is cleared. A one-shot timeout does not need
 * clearing after it fires; an interval does. */
KUI_C_API KuiStatus kui_platform_set_timeout (KuiTaskFn fn, void* user_data,
                                       uint32_t delay_ms, KuiTimerId* out_id);
KUI_C_API KuiStatus kui_platform_set_interval(KuiTaskFn fn, void* user_data,
                                       uint32_t interval_ms, KuiTimerId* out_id);
KUI_C_API KuiStatus kui_platform_clear_timer (KuiTimerId id);
/* Run once when the loop next goes idle. Main thread only. */
KUI_C_API KuiStatus kui_platform_request_idle(KuiTaskFn fn, void* user_data);

/* Hand fn over to the main thread. This is the one function in this header that
 * may be called from ANY thread (worker threads, foreign runtimes' threads); use
 * it to bring results of background work back to the UI instead of polling.
 *
 * fn runs once on the main thread from inside the event loop (kui_platform_run,
 * wait_events or poll_events -- on Windows also inside modal loops such as
 * dragging a window border). It always runs asynchronously, even when posted
 * from the main thread itself; callbacks posted from one thread run in posting
 * order. A loop that is waiting for events is woken immediately.
 *
 * Returns KUI_ERR_NOT_SUPPORTED while the platform is not initialised (before
 * kui_platform_init or once kui_platform_shutdown has started). Then neither fn
 * nor discard_fn is called and user_data stays with the caller.
 *
 * Callbacks still queued when kui_platform_shutdown runs are not executed;
 * discard_fn (optional) is called for each of them instead, from inside
 * kui_platform_shutdown on its thread, so callback state can be released.
 * discard_fn must not call KUI. After KUI_OK exactly one of fn and discard_fn
 * is eventually called -- never both, and never on a thread other than the
 * main thread. */
KUI_C_API KuiStatus kui_platform_post_to_main_thread(KuiTaskFn fn, KuiTaskFn discard_fn,
                                                     void* user_data);
KUI_C_API KuiStatus kui_platform_set_f12_devtools_enabled(int enabled);

typedef struct KuiCMonitorInfo {
    float bounds_x, bounds_y, bounds_width, bounds_height;
    float work_x, work_y, work_width, work_height;
    float scale;
    int is_primary;
} KuiCMonitorInfo;
KUI_C_API KuiStatus kui_platform_get_primary_monitor(KuiCMonitorInfo* out_monitor);

/* ---------------------------------------------------------------------------
 * 2. Window                                                     [IMPLEMENTED]
 *
 * LIFETIME -- a third model, distinct from both of the others in this header:
 *   A window is uniquely owned (the C++ side hands back a unique_ptr), so there
 *   is no retain and kui_window_release DESTROYS the window rather than dropping
 *   a reference. Release each window handle exactly once.
 *
 *   For contrast: Document/Element handles are reference counted (release just
 *   drops your reference), and a KuiEvent* is a borrowed pointer valid only
 *   inside its callback.
 *
 * A window creates and owns its document; there is no way to attach an
 * externally created one, so the draft's kui_window_set_document could not be
 * implemented. Use kui_window_get_document, which hands back a counted
 * reference -- that reference keeps the document alive even after the window is
 * released.
 * ------------------------------------------------------------------------- */
typedef struct {
    const char* title;      /* UTF-8; NULL keeps the library default */
    size_t title_len;       /* bytes; 0 = strlen */
    int    width;           /* 0 = library default */
    int    height;
    int    min_width, min_height;   /* 0 = no constraint */
    int    max_width, max_height;   /* 0 = no limit */
    /* These two are phrased negatively on purpose. The library defaults are
     * "resizable" and "decorated", and every other field here treats 0 as
     * "use the default" -- so a positively-named `resizable` field would make a
     * zero-initialized struct mean "not resizable", and there is no setter to
     * change either one after creation. Negating them keeps memset(0) == the
     * ordinary window while still allowing both to be turned off. */
    int    no_resize;       /* 1 = fixed size */
    int    no_decorations;  /* 1 = no title bar or borders */
    int    transparent;
    int    always_on_top;
    int    maximized;
    int    centered;
    int    frameless;
    int    frameless_shadow;
    float  opacity;         /* 0 is treated as 1.0 (fully opaque) */
} KuiWindowProps;

/* Zero-initialize KuiWindowProps and only set what you care about: every field
 * treats 0 as "use the library default", so memset(&props, 0, sizeof props)
 * yields a normal resizable decorated window. */
/* Measured: the window is already VISIBLE when this returns -- creation shows
 * it, there is no hidden-until-show step. Call kui_window_hide immediately if
 * you need to set things up off-screen first. */
KUI_C_API KuiStatus kui_window_create (const KuiWindowProps* props, KuiWindow** out);
/* Destroys the window. */
KUI_C_API KuiStatus kui_window_release(KuiWindow* w);
KUI_C_API KuiStatus kui_window_show   (KuiWindow* w);
KUI_C_API KuiStatus kui_window_hide   (KuiWindow* w);
KUI_C_API KuiStatus kui_window_is_visible(KuiWindow* w, int* out_visible);

/* Yields a counted reference to the window's own document. */
KUI_C_API KuiStatus kui_window_get_document(KuiWindow* w, KuiDocument** out);

KUI_C_API KuiStatus kui_window_get_title(KuiWindow* w, char* buf, size_t buf_capacity,
                                         size_t* out_len);
KUI_C_API KuiStatus kui_window_set_title(KuiWindow* w, const char* title, size_t len);
KUI_C_API KuiStatus kui_window_get_size(KuiWindow* w, int* out_width, int* out_height);
/* Actually resizes the OS window. */
KUI_C_API KuiStatus kui_window_resize   (KuiWindow* w, int width, int height);
KUI_C_API KuiStatus kui_window_get_position(KuiWindow* w, int* out_x, int* out_y);
KUI_C_API KuiStatus kui_window_set_position(KuiWindow* w, int x, int y);
KUI_C_API KuiStatus kui_window_minimize (KuiWindow* w);
KUI_C_API KuiStatus kui_window_maximize (KuiWindow* w);
KUI_C_API KuiStatus kui_window_restore  (KuiWindow* w);
KUI_C_API KuiStatus kui_window_set_always_on_top(KuiWindow* w, int on_top);
KUI_C_API KuiStatus kui_window_set_opacity(KuiWindow* w, float opacity);
KUI_C_API KuiStatus kui_window_get_device_pixel_ratio(KuiWindow* w, float* out_ratio);
KUI_C_API KuiStatus kui_window_is_minimized(KuiWindow* w, int* out_minimized);
KUI_C_API KuiStatus kui_window_is_maximized(KuiWindow* w, int* out_maximized);
KUI_C_API KuiStatus kui_window_close(KuiWindow* w);
KUI_C_API KuiStatus kui_window_should_close(KuiWindow* w, int* out_should_close);

typedef void (*KuiWindowResizeFn)(uint32_t width, uint32_t height, void* user_data);
/* Replaces the current resize callback. Pass NULL to clear it. */
KUI_C_API KuiStatus kui_window_on_resize(KuiWindow* w, KuiWindowResizeFn fn,
                                         void* user_data);
KUI_C_API KuiStatus kui_window_start_devtools(KuiWindow* w, uint16_t port,
                                              int* out_started);
KUI_C_API KuiStatus kui_window_get_devtools_url(KuiWindow* w, char* buf,
                                                size_t buf_capacity, size_t* out_len);
/* HWND on Windows, the window's content NSView* on macOS (its NSWindow is
 * [view window]), X11 Window on Linux. */
KUI_C_API KuiStatus kui_window_get_native_handle(KuiWindow* w, void** out_handle);

/* ---------------------------------------------------------------------------
 * 3. Document & DOM
 *
 * [IMPLEMENTED] create / retain / release / loading / stylesheets / document
 *               structure / selectors / create_element
 * ------------------------------------------------------------------------- */
KUI_C_API KuiStatus kui_document_create   (KuiDocument** out);
/* Add a reference. Pair every retain with a release. */
KUI_C_API KuiStatus kui_document_retain   (KuiDocument*  doc);
/* Drop a reference. Releasing NULL is an error, not a no-op, so that
 * double-release shows up instead of passing silently. */
KUI_C_API KuiStatus kui_document_release  (KuiDocument*  doc);

KUI_C_API KuiStatus kui_document_load_html       (KuiDocument* doc, const char* html, size_t html_len);
KUI_C_API KuiStatus kui_document_load_html_file  (KuiDocument* doc, const char* path, size_t path_len);
KUI_C_API KuiStatus kui_document_add_stylesheet  (KuiDocument* doc, const char* css,  size_t css_len);
KUI_C_API KuiStatus kui_document_clear_stylesheets(KuiDocument* doc);

KUI_C_API KuiStatus kui_document_root            (KuiDocument* doc, KuiElement** out);
KUI_C_API KuiStatus kui_document_body            (KuiDocument* doc, KuiElement** out);
KUI_C_API KuiStatus kui_document_head            (KuiDocument* doc, KuiElement** out);

KUI_C_API KuiStatus kui_document_get_element_by_id(KuiDocument* doc, const char* id, size_t id_len,
                                         KuiElement** out);
KUI_C_API KuiStatus kui_document_query_selector  (KuiDocument* doc, const char* sel, size_t sel_len,
                                        KuiElement** out);

KUI_C_API KuiStatus kui_document_query_selector_all(KuiDocument* doc, const char* sel, size_t sel_len,
                                          KuiElement** out_buf, size_t buf_capacity,
                                          size_t* out_count);

KUI_C_API KuiStatus kui_document_create_element(KuiDocument* doc, const char* tag, size_t tag_len,
                                      KuiElement** out);

/* ---------------------------------------------------------------------------
 * 4. Element manipulation
 *
 * [IMPLEMENTED] retain / release / tag / id / attributes / text / innerHTML /
 *               class helpers / selectors / traversal / tree mutation
 * ------------------------------------------------------------------------- */
/* Every function that hands back a KuiElement* already gave you a reference;
 * retain only when you want an ADDITIONAL one. Release each reference exactly
 * once. A released handle must not be used again.
 *
 * A handle keeps the element alive on its own, so it stays valid even after the
 * element is removed from the document -- unlike a raw C++ Element*. */
KUI_C_API KuiStatus kui_element_retain (KuiElement* el);
KUI_C_API KuiStatus kui_element_release(KuiElement* el);

KUI_C_API KuiStatus kui_element_get_tag_name(KuiElement* el, char* buf, size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_element_get_id      (KuiElement* el, char* buf, size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_element_get_class_name(KuiElement* el, char* buf,
                                                size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_element_set_class_name(KuiElement* el, const char* value,
                                                size_t value_len);

KUI_C_API KuiStatus kui_element_set_attribute(KuiElement* el, const char* name, size_t name_len,
                                    const char* value, size_t value_len);
KUI_C_API KuiStatus kui_element_get_attribute(KuiElement* el, const char* name, size_t name_len,
                                    char* buf, size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_element_has_attribute(KuiElement* el, const char* name, size_t name_len,
                                    int* out_has);
KUI_C_API KuiStatus kui_element_remove_attribute(KuiElement* el, const char* name, size_t name_len);

/* Current state of a form control -- what the user typed or picked -- as opposed to the content
 * attributes kui_element_get_attribute reads (the initial value="..." / checked / selected).
 *
 * kui_element_get_value / set_value: <input>, <textarea>, <select> and <output> use their value;
 * <progress> and <meter> use the numeric value as decimal text ("%.15g"; set_value parses it,
 * KUI_ERR_INVALID_ARG when it is not a number). Any other element: KUI_ERR_NOT_SUPPORTED (use the
 * attribute). Setting does not dispatch input / change events, like assigning the DOM property.
 * get_value follows the usual two-call string convention (see kui_element_get_attribute). */
KUI_C_API KuiStatus kui_element_get_value(KuiElement* el, char* buf, size_t buf_capacity,
                                          size_t* out_len);
KUI_C_API KuiStatus kui_element_set_value(KuiElement* el, const char* value, size_t value_len);
/* Checked state of an <input> (*out_checked is 0 or 1; meaningful for checkbox and radio).
 * set_checked does not dispatch events. Any other element: KUI_ERR_NOT_SUPPORTED. */
KUI_C_API KuiStatus kui_element_get_checked(KuiElement* el, int* out_checked);
KUI_C_API KuiStatus kui_element_set_checked(KuiElement* el, int checked);
/* Selected option of a <select> (-1 = none). set_selected_index clamps like the DOM and does not
 * dispatch a change event. Any other element: KUI_ERR_NOT_SUPPORTED. */
KUI_C_API KuiStatus kui_element_get_selected_index(KuiElement* el, int32_t* out_index);
KUI_C_API KuiStatus kui_element_set_selected_index(KuiElement* el, int32_t index);

KUI_C_API KuiStatus kui_element_set_text_content(KuiElement* el, const char* text, size_t text_len);
KUI_C_API KuiStatus kui_element_get_text_content(KuiElement* el, char* buf, size_t buf_capacity,
                                       size_t* out_len);
KUI_C_API KuiStatus kui_element_set_inner_html(KuiElement* el, const char* html, size_t html_len);
KUI_C_API KuiStatus kui_element_get_inner_html(KuiElement* el, char* buf, size_t buf_capacity,
                                     size_t* out_len);

KUI_C_API KuiStatus kui_element_class_list_add   (KuiElement* el, const char* token, size_t len);
KUI_C_API KuiStatus kui_element_class_list_remove(KuiElement* el, const char* token, size_t len);
KUI_C_API KuiStatus kui_element_class_list_toggle(KuiElement* el, const char* token, size_t len);
KUI_C_API KuiStatus kui_element_class_list_contains(KuiElement* el, const char* token, size_t len,
                                          int* out_contains);

KUI_C_API KuiStatus kui_element_query_selector(KuiElement* el, const char* sel, size_t sel_len,
                                                KuiElement** out);
KUI_C_API KuiStatus kui_element_query_selector_all(KuiElement* el, const char* sel, size_t sel_len,
                                                    KuiElement** out_buf, size_t buf_capacity,
                                                    size_t* out_count);

KUI_C_API KuiStatus kui_element_parent           (KuiElement* el, KuiElement** out);
KUI_C_API KuiStatus kui_element_first_child      (KuiElement* el, KuiElement** out);
KUI_C_API KuiStatus kui_element_last_child       (KuiElement* el, KuiElement** out);
KUI_C_API KuiStatus kui_element_next_sibling     (KuiElement* el, KuiElement** out);
KUI_C_API KuiStatus kui_element_previous_sibling (KuiElement* el, KuiElement** out);
KUI_C_API KuiStatus kui_element_child_count(KuiElement* el, size_t* out_count);

/* A child already in a tree is moved. KUI_ERR_INVALID_ARG if child is parent itself or
 * one of its ancestors (the tree would become a cycle). */
KUI_C_API KuiStatus kui_element_append_child(KuiElement* parent, KuiElement* child);
KUI_C_API KuiStatus kui_element_remove_child(KuiElement* parent, KuiElement* child);
KUI_C_API KuiStatus kui_element_remove      (KuiElement* el);
KUI_C_API KuiStatus kui_element_set_scroll_position(KuiElement* el, float left, float top);
KUI_C_API KuiStatus kui_element_scroll_by(KuiElement* el, float dx, float dy);
KUI_C_API KuiStatus kui_element_focus(KuiElement* el);
KUI_C_API KuiStatus kui_element_blur(KuiElement* el);

typedef struct {
    float x, y, width, height;
} KuiRect;

/* kui_element_get_bounding_rect and the other geometry accessors live in
 * section 6b, where the layout caveat that applies to all of them is stated
 * once instead of per function. */

/* ---------------------------------------------------------------------------
 * 5. Events                                                    [IMPLEMENTED]
 *
 * LIFETIME, and it differs from every other handle in this header:
 *   A KuiEvent* is valid ONLY for the duration of the callback it is passed to.
 *   Do not store it, and do not release it -- there is no kui_event_release.
 *   The underlying C++ Event is a stack object owned by the dispatcher and is
 *   not reference counted, so a saved pointer dangles the moment the callback
 *   returns. Copy out whatever you need before returning.
 *
 *   Handles obtained FROM an event (kui_event_get_target) are ordinary
 *   reference-counted element handles and DO need releasing.
 *
 * Callbacks run on the main thread, inside dispatch. Mutating the DOM from a
 * listener is allowed, the same as in the C++ API.
 * ------------------------------------------------------------------------- */
typedef void (*KuiEventListenerFn)(KuiEvent* evt, void* user_data);
typedef uint64_t KuiListenerId;

KUI_C_API KuiStatus kui_element_add_event_listener(KuiElement* el, const char* event_name, size_t len,
                                         KuiEventListenerFn fn, void* user_data,
                                         KuiListenerId* out_id);
/* Like kui_element_add_event_listener with a phase: capture != 0 registers a capture-phase listener,
 * which runs while the event travels down from the root to the target (first at the target itself),
 * before any non-capture listener; capture == 0 is the plain listener (target and bubbling phases).
 * stopPropagation in a capture listener ends the descent. Remove either kind with
 * kui_element_remove_event_listener. Additive, 3.61.8. */
KUI_C_API KuiStatus kui_element_add_event_listener_ex(KuiElement* el, const char* event_name,
                                                      size_t len, KuiEventListenerFn fn,
                                                      void* user_data, int capture,
                                                      KuiListenerId* out_id);

/* Removal needs the event name as well as the id: listener ids are scoped per
 * event type in the underlying API, so an id alone is ambiguous. (The earlier
 * draft omitted the name, which could not have been implemented.) */
KUI_C_API KuiStatus kui_element_remove_event_listener(KuiElement* el,
                                         const char* event_name, size_t len,
                                         KuiListenerId id);
/* Remove every listener registered for one event name. */
KUI_C_API KuiStatus kui_element_remove_event_listeners(KuiElement* el,
                                         const char* event_name, size_t len);

/* Dispatch a synthetic event at an element, running the normal capture/bubble
 * path so registered listeners see it. Without this there would be no way to
 * drive events from C at all -- the C++ dispatcher is not otherwise reachable.
 *
 * out_not_canceled (optional) receives 1 when no listener called
 * kui_event_prevent_default, matching the C++ dispatchEvent return.
 *
 * The element must be attached to a document; KUI_ERR_INVALID_ARG otherwise.
 * This carries no coordinates or key state -- see the devtools section for
 * synthetic mouse and keyboard injection. */
KUI_C_API KuiStatus kui_element_dispatch_event(KuiElement* el,
                                         const char* event_name, size_t len,
                                         int bubbles, int cancelable,
                                         int* out_not_canceled);

KUI_C_API KuiStatus kui_event_get_type      (KuiEvent* evt, char* buf, size_t buf_capacity, size_t* out_len);
/* Yields a reference the caller must release. KUI_ERR_NOT_FOUND if the target
 * is not an element (a text node, for instance). */
KUI_C_API KuiStatus kui_event_get_target    (KuiEvent* evt, KuiElement** out);
KUI_C_API KuiStatus kui_event_get_current_target(KuiEvent* evt, KuiElement** out);
/* relatedTarget of mouse events (mouseover / mouseout / mouseenter / mouseleave; for a
 * <kui-context-menu>'s "menu-open" it is the right-clicked element) and focus events
 * (focus / blur). Yields a reference the caller must release. KUI_ERR_NOT_FOUND when the
 * event has none or is of another kind. */
KUI_C_API KuiStatus kui_event_get_related_target(KuiEvent* evt, KuiElement** out);
KUI_C_API KuiStatus kui_event_prevent_default (KuiEvent* evt);
KUI_C_API KuiStatus kui_event_stop_propagation(KuiEvent* evt);
KUI_C_API KuiStatus kui_event_get_flags     (KuiEvent* evt, int* out_bubbles,
                                             int* out_cancelable,
                                             int* out_default_prevented);

/* Mouse accessors. All return KUI_ERR_NOT_SUPPORTED when the event is not a
 * mouse event, so a listener registered for several types can probe safely.
 *
 * Three coordinate spaces are exposed separately rather than behind one vague
 * "get_xy": client is viewport-relative, page includes scroll offset, offset is
 * relative to the target's padding box. */
KUI_C_API KuiStatus kui_event_mouse_get_client_xy(KuiEvent* evt, float* x, float* y);
KUI_C_API KuiStatus kui_event_mouse_get_page_xy  (KuiEvent* evt, float* x, float* y);
KUI_C_API KuiStatus kui_event_mouse_get_offset_xy(KuiEvent* evt, float* x, float* y);
/* out_button receives a KuiCMouseButton value. */
KUI_C_API KuiStatus kui_event_mouse_get_button   (KuiEvent* evt, int* out_button);

/* Keyboard accessors, likewise KUI_ERR_NOT_SUPPORTED for non-key events.
 *   key  -- "a", "Enter", "ArrowUp" (what the keypress means)
 *   code -- "KeyA", "Enter", "ArrowUp" (which physical key)  */
KUI_C_API KuiStatus kui_event_key_get_key (KuiEvent* evt, char* buf, size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_event_key_get_code(KuiEvent* evt, char* buf, size_t buf_capacity, size_t* out_len);
/* Any out_* may be NULL if that modifier is not of interest. */
KUI_C_API KuiStatus kui_event_key_get_modifiers(KuiEvent* evt, int* out_ctrl, int* out_shift,
                                                int* out_alt, int* out_meta);

/* ---------------------------------------------------------------------------
 * 6. Inline style                                              [IMPLEMENTED]
 *
 * This is the element's own `style="..."` declaration, not its computed style.
 *
 * Values come back SERIALIZED, which is not always the string that went in.
 * Measured examples: setting `color: red` reads back as `#ff0000`; lengths keep
 * their unit (`100px` stays `100px`). Compare against the serialized form, or
 * keep your own copy of what you set.
 * ------------------------------------------------------------------------- */
/* important != 0 sets the declaration as !important. */
KUI_C_API KuiStatus kui_element_set_style_property(KuiElement* el, const char* prop, size_t prop_len,
                                         const char* value, size_t value_len,
                                         int important);
/* KUI_ERR_NOT_FOUND when the property is not set on this element. */
KUI_C_API KuiStatus kui_element_get_style_property(KuiElement* el, const char* prop, size_t prop_len,
                                         char* buf, size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_element_remove_style_property(KuiElement* el, const char* prop, size_t prop_len);
KUI_C_API KuiStatus kui_element_style_property_is_important(KuiElement* el,
                                         const char* prop, size_t prop_len,
                                         int* out_important);

/* The whole declaration as text, equivalent to the style attribute's value. */
KUI_C_API KuiStatus kui_element_get_style_text(KuiElement* el, char* buf, size_t buf_capacity,
                                         size_t* out_len);
KUI_C_API KuiStatus kui_element_set_style_text(KuiElement* el, const char* css, size_t css_len);

/* Enumerate the properties actually set, for inspection tools:
 *     kui_element_get_style_property_count(el, &n);
 *     for (i = 0; i < n; ++i)
 *         kui_element_get_style_property_name(el, i, buf, sizeof buf, &len);
 * Indices are only stable until the declaration is modified. */
KUI_C_API KuiStatus kui_element_get_style_property_count(KuiElement* el, size_t* out_count);
KUI_C_API KuiStatus kui_element_get_style_property_name(KuiElement* el, size_t index,
                                         char* buf, size_t buf_capacity, size_t* out_len);

/* ---------------------------------------------------------------------------
 * 6b. Geometry                                                 [IMPLEMENTED]
 *
 * All of these read laid-out geometry, so they are only meaningful after layout
 * has run. On a document that was never attached to a window and never laid out,
 * they succeed and report zeroes rather than failing -- there is no error to
 * report, the boxes genuinely have no size yet.
 *
 * Every out_* pointer is optional; pass NULL for the values you do not need.
 * Passing all-NULL is an error, since the call would have no effect.
 * ------------------------------------------------------------------------- */
KUI_C_API KuiStatus kui_element_get_bounding_rect(KuiElement* el, KuiRect* out);

/* Padding box: what the element's content area actually is (left/top are the
 * border widths, width/height exclude a rendered scrollbar). Like browsers, all
 * four values are whole pixels: floor(x + 0.5) of the exact value. Use
 * kui_element_get_bounding_rect() when sub-pixel geometry is needed. */
KUI_C_API KuiStatus kui_element_get_client_metrics(KuiElement* el, float* out_left, float* out_top,
                                         float* out_width, float* out_height);
/* Border box relative to the offset parent. Like browsers, all four values are
 * whole pixels: left/top are rounded from the exact offset to the offset parent,
 * width/height from the exact border-box size, each as floor(x + 0.5). Use
 * kui_element_get_bounding_rect() when sub-pixel geometry is needed. */
KUI_C_API KuiStatus kui_element_get_offset_metrics(KuiElement* el, float* out_left, float* out_top,
                                         float* out_width, float* out_height);
/* Scroll position and the full scrollable extent. Like browsers, out_width /
 * out_height are whole pixels (floor(x + 0.5) of the exact extent); out_left /
 * out_top are the current scroll offset and are not rounded. */
KUI_C_API KuiStatus kui_element_get_scroll_metrics(KuiElement* el, float* out_left, float* out_top,
                                         float* out_width, float* out_height);

/* ---------------------------------------------------------------------------
 * 7. DevTools automation (implemented in v3.3.0)
 *
 * These functions provide direct (in-process) access to the DevTools command
 * dispatcher, without requiring a WebSocket connection.  Intended for:
 *   - Embedded test harnesses (CI screenshot diff, UI regression)
 *   - Automated interaction drivers (robot-framework style)
 *   - Headless CI pipelines (no browser required)
 *
 * All functions are main-thread-only (same threading rule as the C++ handlers).
 * Do NOT call from background threads.
 * ------------------------------------------------------------------------- */

/* NOTE on why these three enums carry a "C" in their names.
 *
 * KUI has a second, older C ABI: the extension protocol in
 * <kui/extension/kui_extension_api.h>, currently at v11 and in production use by
 * the editor, PDF and 3D extensions. It already defines enums named
 * "KuiMouseEventType", "KuiMouseButton" and "KuiKeyEventType" (no C), and it
 * cannot be changed without breaking every shipped extension.
 *
 * The draft version of this header reused those exact names with different values
 * and different members. Mouse button 0 means LEFT in the extension ABI but NONE
 * in the draft, and the event enums do not even describe the same set of events
 * (element-level enter/leave there; synthetic injection with a wheel event here).
 * Same name, different meaning is the one collision that must never ship: a
 * translation unit seeing both headers would silently reinterpret the values.
 *
 * Hence the C in KuiCMouseButton and the KUI_C_ prefix on the enumerators. The
 * two ABIs keep separate namespaces, and the distinct spellings make it obvious
 * at any call site which protocol is in play.
 */
typedef enum KuiCMouseEventType {
    KUI_C_MOUSE_MOVED    = 0,
    KUI_C_MOUSE_PRESSED  = 1,
    KUI_C_MOUSE_RELEASED = 2,
    KUI_C_MOUSE_WHEEL    = 3,
} KuiCMouseEventType;

typedef enum KuiCMouseButton {
    KUI_C_MOUSE_BUTTON_NONE   = 0,
    KUI_C_MOUSE_BUTTON_LEFT   = 1,
    KUI_C_MOUSE_BUTTON_MIDDLE = 2,
    KUI_C_MOUSE_BUTTON_RIGHT  = 3,
} KuiCMouseButton;

typedef enum KuiCKeyEventType {
    KUI_C_KEY_DOWN = 0,
    KUI_C_KEY_UP   = 1,
    KUI_C_KEY_CHAR = 2,
} KuiCKeyEventType;

/**
 * Inject a synthetic mouse event at (x, y) in viewport coordinates.
 *
 * @param doc        Target document.
 * @param type       KUI_C_MOUSE_MOVED | KUI_C_MOUSE_PRESSED | KUI_C_MOUSE_RELEASED | KUI_C_MOUSE_WHEEL
 * @param x          X coordinate (CSS pixels, viewport-relative).
 * @param y          Y coordinate (CSS pixels, viewport-relative).
 * @param button     KUI_C_MOUSE_BUTTON_LEFT | _MIDDLE | _RIGHT | _NONE
 * @param modifiers  Bitmask: 1=Alt, 2=Ctrl, 4=Meta, 8=Shift
 * @param click_count  1 for click, 2 for double-click.
 * @param delta_x    Wheel X delta (KUI_C_MOUSE_WHEEL only, otherwise 0).
 * @param delta_y    Wheel Y delta (KUI_C_MOUSE_WHEEL only, otherwise 0).
 * @return KUI_OK on success; KUI_ERR_INVALID_ARG for a null document, a negative
 *         click_count or an unknown event type / button / modifier bit;
 *         KUI_ERR_NOT_FOUND if the document has no event dispatcher.
 *         Coordinates are not range-checked.
 */
KUI_C_API KuiStatus kui_devtools_dispatch_mouse_event(
    KuiDocument*      doc,
    KuiCMouseEventType type,
    float             x,
    float             y,
    KuiCMouseButton    button,
    int               modifiers,
    int               click_count,
    float             delta_x,
    float             delta_y
);

/**
 * Inject a synthetic keyboard event to the currently focused element.
 *
 * @param doc          Target document.
 * @param type         KUI_C_KEY_DOWN | KUI_C_KEY_UP | KUI_C_KEY_CHAR
 * @param key          DOM key name (UTF-8, e.g. "Enter", "ArrowDown", "a").
 * @param key_len      Byte length of key; 0 = null-terminated.
 * @param code         Physical key code (UTF-8, e.g. "KeyA", "Enter"); may be NULL.
 * @param code_len     Byte length of code; 0 = null-terminated.
 * @param modifiers    Same bitmask as dispatchMouseEvent.
 * @param text         Character to input (KUI_C_KEY_CHAR only); may be NULL otherwise.
 * @param text_len     Byte length of text; 0 = null-terminated.
 * @param is_auto_repeat  Non-zero for held-key repeat events.
 * @return KUI_OK on success.
 */
KUI_C_API KuiStatus kui_devtools_dispatch_key_event(
    KuiDocument*    doc,
    KuiCKeyEventType type,
    const char*     key,        size_t key_len,
    const char*     code,       size_t code_len,
    int             modifiers,
    const char*     text,       size_t text_len,
    int             is_auto_repeat
);

/**
 * Retrieve the four-vertex box model for the element matching selector.
 *
 * Writes 32 floats into out_quads (content, padding, border, margin × 8 floats each),
 * and the element width/height into out_width / out_height.
 *
 * @param doc        Target document.
 * @param selector   CSS selector (UTF-8, null-terminated).
 * @param out_quads  Output buffer for 32 floats [content(8), padding(8), border(8), margin(8)].
 * @param out_width  Output: element width in CSS pixels.
 * @param out_height Output: element height in CSS pixels.
 * @return KUI_OK on success; KUI_ERR_INVALID_ARG for a null argument;
 *         KUI_ERR_NOT_FOUND if selector matches nothing.
 */
KUI_C_API KuiStatus kui_devtools_get_box_model(
    KuiDocument* doc,
    const char*  selector,
    float        out_quads[32],
    float*       out_width,
    float*       out_height
);

/**
 * Execute a raw CDP JSON command in-process and receive the JSON response.
 *
 * Equivalent to sending the command over WebSocket but without network overhead.
 * The response string is written into buf; out_len receives the actual byte count.
 * If buf_capacity is too small, the response is truncated and out_len reflects the
 * full required capacity (caller should retry with a larger buffer).
 *
 * @param doc           Target document.
 * @param json_command  Null-terminated JSON-RPC command string.
 * @param buf           Output buffer for the JSON response.
 * @param buf_capacity  Byte capacity of buf.
 * @param out_len       Output: actual response length (may exceed buf_capacity).
 * @return KUI_OK on success; KUI_ERR_BUFFER_TOO_SMALL if truncated.
 */
KUI_C_API KuiStatus kui_devtools_dispatch_command(
    KuiDocument* doc,
    const char*  json_command,
    char*        buf,
    size_t       buf_capacity,
    size_t*      out_len
);

/* ---------------------------------------------------------------------------
 * 8. Windows x64 Full Profile (additive product-major-3 surface)
 *
 * These APIs deliberately reuse opaque handles and caller-owned buffers. They
 * expose the native SDK facilities used by the networking, Lua, extension,
 * media, Canvas/WebGL, PDF and 3D examples without passing STL, exceptions or
 * C++ vtables across the DLL boundary.
 * ------------------------------------------------------------------------- */

typedef struct KuiScriptEngine    KuiScriptEngine;
typedef struct KuiHttpClient      KuiHttpClient;
typedef struct KuiHttpResponse    KuiHttpResponse;
typedef struct KuiExtensionResult KuiExtensionResult;

/* -- Platform services ---------------------------------------------------- */
KUI_C_API KuiStatus kui_platform_get_monitor_count(size_t* out_count);
KUI_C_API KuiStatus kui_platform_get_monitor(size_t index,
                                              KuiCMonitorInfo* out_monitor,
                                              char* name_buf, size_t name_capacity,
                                              size_t* out_name_len);
KUI_C_API KuiStatus kui_platform_get_primary_monitor_physical_size(float* out_width,
                                                                    float* out_height);
KUI_C_API KuiStatus kui_platform_get_clipboard_text(char* buf, size_t capacity,
                                                     size_t* out_len);
KUI_C_API KuiStatus kui_platform_set_clipboard_text(const char* text, size_t len);
KUI_C_API KuiStatus kui_platform_has_clipboard_text(int* out_has_text);
KUI_C_API KuiStatus kui_platform_get_os_name(char* buf, size_t capacity, size_t* out_len);
KUI_C_API KuiStatus kui_platform_get_os_version(char* buf, size_t capacity, size_t* out_len);

typedef struct KuiCFileDialogOptions {
    const char* title;
    size_t title_len;
    const char* default_path;
    size_t default_path_len;
    const char* const* filters;
    size_t filter_count;
    int allow_multiple;
} KuiCFileDialogOptions;

/* File-dialog output is a one-shot operation. A 32768-byte UTF-8 buffer is
 * sufficient for the Windows path namespace; cancellation returns NOT_FOUND. */
KUI_C_API KuiStatus kui_platform_open_file_dialog(const KuiCFileDialogOptions* options,
                                                   char* buf, size_t capacity,
                                                   size_t* out_len);
KUI_C_API KuiStatus kui_platform_save_file_dialog(const KuiCFileDialogOptions* options,
                                                   char* buf, size_t capacity,
                                                   size_t* out_len);
KUI_C_API KuiStatus kui_platform_select_folder_dialog(const char* title, size_t title_len,
                                                       char* buf, size_t capacity,
                                                       size_t* out_len);

/* -- Window Full Profile -------------------------------------------------- */
KUI_C_API KuiStatus kui_window_set_frameless_sizeable(KuiWindow* window, int enabled);
KUI_C_API KuiStatus kui_window_set_frame_radius(KuiWindow* window, float radius);
KUI_C_API KuiStatus kui_window_clear_frame_radius(KuiWindow* window);
KUI_C_API KuiStatus kui_window_get_devtools_port(KuiWindow* window, uint16_t* out_port);

typedef void (*KuiWindowEventFn)(KuiEvent* event, void* user_data);
KUI_C_API KuiStatus kui_window_add_event_listener(KuiWindow* window,
                                                   const char* event_name, size_t len,
                                                   KuiWindowEventFn fn, void* user_data,
                                                   int once, KuiListenerId* out_id);
KUI_C_API KuiStatus kui_window_remove_event_listener(KuiWindow* window,
                                                      const char* event_name, size_t len,
                                                      KuiListenerId id);
KUI_C_API KuiStatus kui_window_remove_event_listeners(KuiWindow* window,
                                                       const char* event_name, size_t len);

/* -- Event extensions ----------------------------------------------------- */
typedef enum KuiCEventTypeTag {
    KUI_C_EVENT_BASE = 0,
    KUI_C_EVENT_MOUSE = 1,
    KUI_C_EVENT_WHEEL = 2,
    KUI_C_EVENT_POINTER = 3,
    KUI_C_EVENT_KEYBOARD = 4,
    KUI_C_EVENT_FOCUS = 5,
    KUI_C_EVENT_INPUT = 6,
    KUI_C_EVENT_DRAG = 7
} KuiCEventTypeTag;
KUI_C_API KuiStatus kui_event_get_type_tag(KuiEvent* event, KuiCEventTypeTag* out_tag);
KUI_C_API KuiStatus kui_event_drag_get_file_count(KuiEvent* event, size_t* out_count);
KUI_C_API KuiStatus kui_event_drag_get_file(KuiEvent* event, size_t index,
                                             char* buf, size_t capacity, size_t* out_len);

/* -- Element specializations --------------------------------------------- */
typedef struct KuiCPositioningOptions {
    int flip;
    int clamp_to_viewport;
    float offset;
    float viewport_margin;
} KuiCPositioningOptions;
KUI_C_API KuiStatus kui_element_position_relative_to(KuiElement* element,
                                                      KuiElement* anchor,
                                                      const char* placement,
                                                      size_t placement_len,
                                                      const KuiCPositioningOptions* options);

KUI_C_API KuiStatus kui_media_play(KuiElement* element);
KUI_C_API KuiStatus kui_media_pause(KuiElement* element);
KUI_C_API KuiStatus kui_media_set_src(KuiElement* element, const char* src, size_t len);
KUI_C_API KuiStatus kui_media_set_controls(KuiElement* element, int enabled);
KUI_C_API KuiStatus kui_media_set_current_time(KuiElement* element, double seconds);
KUI_C_API KuiStatus kui_media_get_current_time(KuiElement* element, double* out_seconds);
KUI_C_API KuiStatus kui_media_get_duration(KuiElement* element, double* out_seconds);
KUI_C_API KuiStatus kui_media_set_volume(KuiElement* element, float volume);
KUI_C_API KuiStatus kui_media_get_volume(KuiElement* element, float* out_volume);
KUI_C_API KuiStatus kui_media_set_muted(KuiElement* element, int muted);
KUI_C_API KuiStatus kui_media_get_muted(KuiElement* element, int* out_muted);
KUI_C_API KuiStatus kui_media_get_paused(KuiElement* element, int* out_paused);
KUI_C_API KuiStatus kui_media_get_ready_state(KuiElement* element, int* out_state);
KUI_C_API KuiStatus kui_media_get_network_state(KuiElement* element, int* out_state);
KUI_C_API KuiStatus kui_media_is_available(KuiElement* element, int* out_available);

KUI_C_API KuiStatus kui_lottie_set_src(KuiElement* element, const char* src, size_t len);
KUI_C_API KuiStatus kui_lottie_play(KuiElement* element);
KUI_C_API KuiStatus kui_lottie_pause(KuiElement* element);
KUI_C_API KuiStatus kui_lottie_get_complete(KuiElement* element, int* out_complete);
KUI_C_API KuiStatus kui_lottie_get_natural_size(KuiElement* element,
                                                 float* out_width, float* out_height);

/* -- Script engine (Lua; also powers Canvas2D and WebGL examples) ---------- */
typedef void (*KuiScriptErrorFn)(const char* error, size_t error_len,
                                 const char* source, size_t source_len,
                                 int line, void* user_data);
KUI_C_API KuiStatus kui_script_engine_create(KuiScriptEngine** out_engine);
KUI_C_API KuiStatus kui_script_engine_release(KuiScriptEngine* engine);
KUI_C_API KuiStatus kui_script_engine_bind_document(KuiScriptEngine* engine,
                                                     KuiDocument* document);
KUI_C_API KuiStatus kui_script_engine_unbind_document(KuiScriptEngine* engine);
KUI_C_API KuiStatus kui_script_engine_eval(KuiScriptEngine* engine,
                                            const char* code, size_t code_len,
                                            const char* source, size_t source_len,
                                            int* out_success,
                                            char* error_buf, size_t error_capacity,
                                            size_t* out_error_len);
KUI_C_API KuiStatus kui_script_engine_execute_scripts(KuiScriptEngine* engine);
KUI_C_API KuiStatus kui_script_engine_set_error_handler(KuiScriptEngine* engine,
                                                         KuiScriptErrorFn fn,
                                                         void* user_data);
KUI_C_API KuiStatus kui_window_set_script_engine(KuiWindow* window,
                                                  KuiScriptEngine* engine);

/* -- HTTP ----------------------------------------------------------------- */
typedef void (*KuiHttpResponseFn)(KuiHttpResponse* response, void* user_data);
KUI_C_API KuiStatus kui_http_client_create(KuiHttpClient** out_client);
KUI_C_API KuiStatus kui_http_client_release(KuiHttpClient* client);
KUI_C_API KuiStatus kui_http_client_set_user_agent(KuiHttpClient* client,
                                                    const char* user_agent, size_t len);
KUI_C_API KuiStatus kui_http_client_fetch_sync(KuiHttpClient* client,
                                                const char* url, size_t len,
                                                double timeout_seconds,
                                                KuiHttpResponse** out_response);
/* fn runs on an HTTP worker thread, NOT the main thread. It owns the response
 * handle and may read and release it there, but anything else (DOM, windows,
 * timers) must be handed to the main thread with kui_platform_post_to_main_thread
 * -- or use kui_http_client_fetch_async_on_main_thread instead. fn receives NULL
 * only if the response object cannot be allocated. Cancelled or failed requests
 * still call fn once, with the error in the response. */
KUI_C_API KuiStatus kui_http_client_fetch_async(KuiHttpClient* client,
                                                 const char* url, size_t len,
                                                 double timeout_seconds,
                                                 KuiHttpResponseFn fn, void* user_data);
/* Same request, but fn runs on the main thread from inside the event loop (see
 * kui_platform_post_to_main_thread), so it may use any KUI API directly. fn owns
 * the response handle. It is called once with NULL if the response cannot be
 * allocated, or -- from inside kui_platform_shutdown -- if the result is still
 * queued when the platform shuts down; then it must not call KUI. If the request
 * finishes after kui_platform_shutdown has started, fn is never called (the
 * process is ending; user_data is not released). Returns KUI_ERR_NOT_SUPPORTED
 * while the platform is not initialised. fn is only ever called on the main
 * thread. */
KUI_C_API KuiStatus kui_http_client_fetch_async_on_main_thread(KuiHttpClient* client,
                                                                const char* url, size_t len,
                                                                double timeout_seconds,
                                                                KuiHttpResponseFn fn,
                                                                void* user_data);
KUI_C_API KuiStatus kui_http_client_cancel_all(KuiHttpClient* client);
KUI_C_API KuiStatus kui_http_response_release(KuiHttpResponse* response);
KUI_C_API KuiStatus kui_http_response_get_status(KuiHttpResponse* response,
                                                  int* out_status_code);
KUI_C_API KuiStatus kui_http_response_get_body(KuiHttpResponse* response,
                                                uint8_t* buf, size_t capacity,
                                                size_t* out_len);
KUI_C_API KuiStatus kui_http_response_get_content_type(KuiHttpResponse* response,
                                                        char* buf, size_t capacity,
                                                        size_t* out_len);
KUI_C_API KuiStatus kui_http_response_get_error(KuiHttpResponse* response,
                                                 char* buf, size_t capacity,
                                                 size_t* out_len);

/* -- Dynamic extensions and the standalone 3D protocol ------------------- */
typedef enum KuiCExtensionValueType {
    KUI_C_EXT_NIL = 0,
    KUI_C_EXT_INT = 1,
    KUI_C_EXT_FLOAT = 2,
    KUI_C_EXT_STRING = 3,
    KUI_C_EXT_BOOL = 4
} KuiCExtensionValueType;
typedef struct KuiCExtensionValue {
    KuiCExtensionValueType type;
    int64_t int_value;
    double float_value;
    const char* string_value;
    size_t string_len;
    int bool_value;
} KuiCExtensionValue;
typedef struct KuiCExtensionPerfStats {
    double last_tick_ms, last_paint_ms, max_tick_ms, max_paint_ms;
    uint64_t tick_count, paint_count;
    double total_tick_ms, total_paint_ms;
} KuiCExtensionPerfStats;

KUI_C_API KuiStatus kui_document_init_extensions(KuiDocument* document,
                                                  const char* directory, size_t len,
                                                  size_t* out_loaded_count);
KUI_C_API KuiStatus kui_document_extension_is_loaded(KuiDocument* document,
                                                      const char* tag_name, size_t len,
                                                      int* out_loaded);
KUI_C_API KuiStatus kui_document_extension_invoke(KuiDocument* document,
                                                   KuiElement* element,
                                                   const char* method, size_t method_len,
                                                   const KuiCExtensionValue* args,
                                                   size_t arg_count, size_t max_results,
                                                   KuiExtensionResult** out_result);
KUI_C_API KuiStatus kui_extension_result_release(KuiExtensionResult* result);
/* A KUI_OK from kui_document_extension_invoke only means the result object was
 * produced; whether the call itself succeeded is out_status here:
 *   0 success, 1 null element, 2 element of another document, 3 method not found,
 *   4 wrong argument count, 5 too many results, 6 the extension callback failed.
 * kui_extension_result_get_error carries the message for non-zero values. */
KUI_C_API KuiStatus kui_extension_result_get_status(KuiExtensionResult* result,
                                                     int* out_status);
KUI_C_API KuiStatus kui_extension_result_get_count(KuiExtensionResult* result,
                                                    size_t* out_count);
KUI_C_API KuiStatus kui_extension_result_get_value(KuiExtensionResult* result,
                                                    size_t index,
                                                    KuiCExtensionValue* out_value);
KUI_C_API KuiStatus kui_extension_result_get_string(KuiExtensionResult* result,
                                                     size_t index,
                                                     char* buf, size_t capacity,
                                                     size_t* out_len);
KUI_C_API KuiStatus kui_extension_result_get_error(KuiExtensionResult* result,
                                                    char* buf, size_t capacity,
                                                    size_t* out_len);
KUI_C_API KuiStatus kui_document_extension_get_perf_stats(KuiDocument* document,
                                                           const char* tag_name, size_t len,
                                                           KuiCExtensionPerfStats* out_stats);
KUI_C_API KuiStatus kui_document_extension_reset_perf_stats(KuiDocument* document);

/* -- Custom elements ------------------------------------------------------ */
typedef void (*KuiCustomElementLifecycleFn)(KuiElement* element, void* user_data);
typedef void (*KuiCustomElementAttributeFn)(KuiElement* element,
                                            const char* name, size_t name_len,
                                            const char* old_value, size_t old_len,
                                            const char* new_value, size_t new_len,
                                            void* user_data);
typedef struct KuiCCustomElementCallbacks {
    KuiCustomElementLifecycleFn connected;
    KuiCustomElementLifecycleFn disconnected;
    KuiCustomElementAttributeFn attribute_changed;
    const char* const* observed_attributes;
    size_t observed_attribute_count;
    void* user_data;
} KuiCCustomElementCallbacks;
KUI_C_API KuiStatus kui_document_define_custom_element(KuiDocument* document,
                                                        const char* tag_name, size_t len,
                                                        const KuiCCustomElementCallbacks* callbacks);
KUI_C_API KuiStatus kui_document_custom_element_is_defined(KuiDocument* document,
                                                            const char* tag_name, size_t len,
                                                            int* out_defined);

/* -- PDF ------------------------------------------------------------------ */
typedef struct KuiCPDFOptions {
    const char* title;
    const char* author;
    const char* subject;
    const char* keywords;
    const char* creator;
    const char* creation_date;
    float page_width_pt;
    float page_height_pt;
    int rasterize_fonts;
    int generate_outline;
} KuiCPDFOptions;
KUI_C_API KuiStatus kui_document_print_to_pdf(KuiDocument* document,
                                               const char* path, size_t path_len,
                                               const KuiCPDFOptions* options,
                                               int* out_written);

/* ---------------------------------------------------------------------------
 * 9. Common window, DOM and platform operations (additive, 3.61.8)
 *
 * Window close / focus notifications are not repeated here: register "close"
 * (cancelable -- kui_event_prevent_default keeps the window open), "focus" or
 * "blur" with kui_window_add_event_listener. Element listeners have no capture
 * option because the native API has none either.
 * ------------------------------------------------------------------------- */

/* -- Window: focus, cursor, fullscreen, frame ------------------------------ */
/* Values match the native CursorType (CSS cursor keywords). */
typedef enum KuiCCursor {
    KUI_C_CURSOR_AUTO = 0, KUI_C_CURSOR_DEFAULT, KUI_C_CURSOR_POINTER,
    KUI_C_CURSOR_CROSSHAIR, KUI_C_CURSOR_TEXT, KUI_C_CURSOR_MOVE,
    KUI_C_CURSOR_NOT_ALLOWED, KUI_C_CURSOR_GRAB, KUI_C_CURSOR_GRABBING,
    KUI_C_CURSOR_WAIT, KUI_C_CURSOR_PROGRESS, KUI_C_CURSOR_HELP,
    KUI_C_CURSOR_CONTEXT_MENU,
    KUI_C_CURSOR_N_RESIZE, KUI_C_CURSOR_E_RESIZE, KUI_C_CURSOR_S_RESIZE,
    KUI_C_CURSOR_W_RESIZE, KUI_C_CURSOR_NE_RESIZE, KUI_C_CURSOR_NW_RESIZE,
    KUI_C_CURSOR_SE_RESIZE, KUI_C_CURSOR_SW_RESIZE, KUI_C_CURSOR_EW_RESIZE,
    KUI_C_CURSOR_NS_RESIZE, KUI_C_CURSOR_NESW_RESIZE, KUI_C_CURSOR_NWSE_RESIZE,
    KUI_C_CURSOR_COL_RESIZE, KUI_C_CURSOR_ROW_RESIZE
} KuiCCursor;
KUI_C_API KuiStatus kui_window_focus(KuiWindow* window);
KUI_C_API KuiStatus kui_window_has_focus(KuiWindow* window, int* out_focused);
/* Switches the cursor immediately; the next hover change re-applies the CSS
 * `cursor` of the content under the pointer (as with script-set cursors). */
KUI_C_API KuiStatus kui_window_set_cursor(KuiWindow* window, KuiCCursor cursor);
KUI_C_API KuiStatus kui_window_set_cursor_visible(KuiWindow* window, int visible);
KUI_C_API KuiStatus kui_window_set_fullscreen(KuiWindow* window, int fullscreen);
KUI_C_API KuiStatus kui_window_is_fullscreen(KuiWindow* window, int* out_fullscreen);
/* Switch the frameless mode and its shadow after creation. */
KUI_C_API KuiStatus kui_window_set_frameless(KuiWindow* window, int frameless);
KUI_C_API KuiStatus kui_window_set_frameless_shadow(KuiWindow* window, int shadow);
/* Any out_* may be NULL. */
KUI_C_API KuiStatus kui_window_get_frame_state(KuiWindow* window, int* out_frameless,
                                                int* out_shadow, int* out_sizeable);
/* Per-corner radii in CSS px (kui_window_set_frame_radius sets all four). */
KUI_C_API KuiStatus kui_window_set_frame_radii(KuiWindow* window, float top_left,
                                                float top_right, float bottom_right,
                                                float bottom_left);
/* KUI_ERR_NOT_FOUND when no radius is set. */
KUI_C_API KuiStatus kui_window_get_frame_radii(KuiWindow* window, float* out_top_left,
                                                float* out_top_right,
                                                float* out_bottom_right,
                                                float* out_bottom_left);
/* Viewport pixels; moves the IME candidate window to follow a custom caret. */
KUI_C_API KuiStatus kui_window_set_ime_candidate_pos(KuiWindow* window, float x, float y);

/* -- DOM: tree editing, selectors, serialization -------------------------- */
/* Inserts child before ref_child; ref_child NULL appends. A child that is already
 * in a tree is moved. KUI_ERR_NOT_FOUND if ref_child is not a child of parent. */
KUI_C_API KuiStatus kui_element_insert_before(KuiElement* parent, KuiElement* child,
                                               KuiElement* ref_child);
/* KUI_ERR_NOT_FOUND if old_child is not a child of parent. */
KUI_C_API KuiStatus kui_element_replace_child(KuiElement* parent, KuiElement* new_child,
                                               KuiElement* old_child);
/* A detached copy; deep also copies the subtree. Yields a counted reference. */
KUI_C_API KuiStatus kui_element_clone(KuiElement* el, int deep, KuiElement** out);
KUI_C_API KuiStatus kui_element_matches(KuiElement* el, const char* sel, size_t sel_len,
                                         int* out_matches);
/* Nearest inclusive ancestor matching sel; KUI_ERR_NOT_FOUND if none. */
KUI_C_API KuiStatus kui_element_closest(KuiElement* el, const char* sel, size_t sel_len,
                                         KuiElement** out);
KUI_C_API KuiStatus kui_element_get_outer_html(KuiElement* el, char* buf,
                                                size_t buf_capacity, size_t* out_len);
/* getComputedStyle(el).getPropertyValue(name): the resolved value of a CSS
 * property by its kebab-case name ("display", "margin-inline-start" ...),
 * the same as Lua's getComputedStyle. Flushes pending style / layout first.
 * An unknown name or an element outside its document yields "" (KUI_OK). */
KUI_C_API KuiStatus kui_element_get_computed_style_value(KuiElement* el, const char* name,
                                                         size_t name_len, char* buf,
                                                         size_t buf_capacity,
                                                         size_t* out_len);
/* position: "beforebegin", "afterbegin", "beforeend" or "afterend". */
KUI_C_API KuiStatus kui_element_insert_adjacent_html(KuiElement* el,
                                                      const char* position, size_t position_len,
                                                      const char* html, size_t html_len);
/* Appends a text node (the C ABI has no handle type for text nodes). */
KUI_C_API KuiStatus kui_element_append_text(KuiElement* el, const char* text, size_t len);
KUI_C_API KuiStatus kui_element_has_focus(KuiElement* el, int* out_focused);
/* Pointer capture (Pointer Events): pointer moves and releases for pointer_id
 * keep going to el until released. Pointer id 1 is the mouse. */
KUI_C_API KuiStatus kui_element_set_pointer_capture(KuiElement* el, int pointer_id);
KUI_C_API KuiStatus kui_element_release_pointer_capture(KuiElement* el, int pointer_id);
KUI_C_API KuiStatus kui_element_has_pointer_capture(KuiElement* el, int pointer_id,
                                                     int* out_captured);
/* Topmost element at a viewport point; KUI_ERR_NOT_FOUND if none. */
KUI_C_API KuiStatus kui_document_element_from_point(KuiDocument* doc, float x, float y,
                                                     KuiElement** out);
/* Same buffer protocol as kui_document_query_selector_all. class_name may hold
 * several space-separated classes (all must match). */
KUI_C_API KuiStatus kui_document_get_elements_by_class_name(KuiDocument* doc,
                                                             const char* class_name, size_t len,
                                                             KuiElement** out_buf,
                                                             size_t buf_capacity,
                                                             size_t* out_count);
KUI_C_API KuiStatus kui_document_get_elements_by_tag_name(KuiDocument* doc,
                                                           const char* tag_name, size_t len,
                                                           KuiElement** out_buf,
                                                           size_t buf_capacity,
                                                           size_t* out_count);

/* -- Platform: message box, URL, clock, monitor lookup -------------------- */
typedef enum KuiCMessageBoxType {
    KUI_C_MESSAGE_BOX_INFO = 0, KUI_C_MESSAGE_BOX_WARNING = 1,
    KUI_C_MESSAGE_BOX_ERROR = 2, KUI_C_MESSAGE_BOX_QUESTION = 3
} KuiCMessageBoxType;
typedef enum KuiCMessageBoxButtons {
    KUI_C_MESSAGE_BOX_OK = 0, KUI_C_MESSAGE_BOX_OK_CANCEL = 1,
    KUI_C_MESSAGE_BOX_YES_NO = 2, KUI_C_MESSAGE_BOX_YES_NO_CANCEL = 3
} KuiCMessageBoxButtons;
typedef enum KuiCMessageBoxResult {
    KUI_C_MESSAGE_BOX_RESULT_OK = 0, KUI_C_MESSAGE_BOX_RESULT_CANCEL = 1,
    KUI_C_MESSAGE_BOX_RESULT_YES = 2, KUI_C_MESSAGE_BOX_RESULT_NO = 3
} KuiCMessageBoxResult;
/* Modal; blocks until the user answers. out_result may be NULL. */
KUI_C_API KuiStatus kui_platform_message_box(const char* title, size_t title_len,
                                              const char* message, size_t message_len,
                                              KuiCMessageBoxType type,
                                              KuiCMessageBoxButtons buttons,
                                              KuiCMessageBoxResult* out_result);
/* Opens url with the system's default handler (browser, mail client...). */
KUI_C_API KuiStatus kui_platform_open_url(const char* url, size_t len);
/* Monotonic milliseconds since an arbitrary origin (the clock timers use). */
KUI_C_API KuiStatus kui_platform_now(double* out_ms);
/* The monitor containing a point in virtual-screen coordinates, or a window given
 * by its native handle (kui_window_get_native_handle), like the native API. */
KUI_C_API KuiStatus kui_platform_get_monitor_from_point(float x, float y,
                                                         KuiCMonitorInfo* out_monitor,
                                                         char* name_buf, size_t name_capacity,
                                                         size_t* out_name_len);
KUI_C_API KuiStatus kui_platform_get_monitor_from_window(void* native_window,
                                                          KuiCMonitorInfo* out_monitor,
                                                          char* name_buf, size_t name_capacity,
                                                          size_t* out_name_len);

/* ---------------------------------------------------------------------------
 * 10. Desktop integration (additive, 3.61.8): native menu bar, tray icon,
 *     keyboard shortcuts, toasts, screenshots
 *
 * Menus are described by plain structs. Every item carries an id chosen by the
 * caller; activating an item calls the menu's KuiMenuActivateFn with that id on
 * the main thread. The definition is copied, so the structs may be freed after
 * the call. kui_window_menu_set_checked / set_enabled update the copy and
 * rebuild the native menu. Callback user_data is never freed by KUI; it stays
 * referenced until the menu / tray / shortcut is replaced, removed or the
 * window is released.
 * ------------------------------------------------------------------------- */
typedef struct KuiCMenuItem {
    const char* text;               /* "&Open" -- & marks the mnemonic */
    size_t text_len;
    const char* shortcut_label;     /* display only, e.g. "Ctrl+O"; may be NULL */
    size_t shortcut_label_len;
    uint32_t id;                    /* passed to the activate callback */
    int separator;
    int disabled;
    int checkable;
    int checked;
    const struct KuiCMenuItem* children;  /* non-empty = submenu */
    size_t child_count;
} KuiCMenuItem;
typedef struct KuiCMenu {
    const char* text;               /* top-level title, e.g. "&File" */
    size_t text_len;
    const KuiCMenuItem* items;
    size_t item_count;
} KuiCMenu;
typedef void (*KuiMenuActivateFn)(uint32_t item_id, void* user_data);

/* Replaces the window's native menu bar. */
KUI_C_API KuiStatus kui_window_set_menu_bar(KuiWindow* window, const KuiCMenu* menus,
                                             size_t menu_count, KuiMenuActivateFn fn,
                                             void* user_data);
KUI_C_API KuiStatus kui_window_clear_menu_bar(KuiWindow* window);
/* KUI_ERR_NOT_FOUND if no item with item_id exists in the current menu bar. */
KUI_C_API KuiStatus kui_window_menu_set_checked(KuiWindow* window, uint32_t item_id,
                                                 int checked);
KUI_C_API KuiStatus kui_window_menu_set_enabled(KuiWindow* window, uint32_t item_id,
                                                 int enabled);

/* -- Tray icon (one per window) ------------------------------------------- */
typedef enum KuiCTrayEvent {
    KUI_C_TRAY_CLICK = 0, KUI_C_TRAY_DOUBLE_CLICK = 1, KUI_C_TRAY_RIGHT_CLICK = 2,
    KUI_C_TRAY_BALLOON_CLICK = 3
} KuiCTrayEvent;
typedef void (*KuiTrayEventFn)(KuiCTrayEvent event, void* user_data);
/* icon_path NULL / empty = the application icon. KUI_ERR_NOT_SUPPORTED when the
 * platform has no tray (or the shell refused the icon). */
KUI_C_API KuiStatus kui_window_tray_create(KuiWindow* window, const char* icon_path,
                                            size_t icon_path_len, const char* tooltip,
                                            size_t tooltip_len, int visible);
KUI_C_API KuiStatus kui_window_tray_destroy(KuiWindow* window);
KUI_C_API KuiStatus kui_window_tray_is_active(KuiWindow* window, int* out_active);
KUI_C_API KuiStatus kui_window_tray_set_visible(KuiWindow* window, int visible);
KUI_C_API KuiStatus kui_window_tray_is_visible(KuiWindow* window, int* out_visible);
KUI_C_API KuiStatus kui_window_tray_set_tooltip(KuiWindow* window, const char* tooltip,
                                                 size_t len);
KUI_C_API KuiStatus kui_window_tray_set_icon(KuiWindow* window, const char* icon_path,
                                              size_t len);
/* Right-click menu: the items of a single menu (same activate protocol). */
KUI_C_API KuiStatus kui_window_tray_set_context_menu(KuiWindow* window,
                                                      const KuiCMenuItem* items,
                                                      size_t item_count, KuiMenuActivateFn fn,
                                                      void* user_data);
KUI_C_API KuiStatus kui_window_tray_popup_context_menu(KuiWindow* window);
/* icon_type: 0 none, 1 info, 2 warning, 3 error (native TrayIcon::showBalloon). */
KUI_C_API KuiStatus kui_window_tray_show_balloon(KuiWindow* window, const char* title,
                                                  size_t title_len, const char* message,
                                                  size_t message_len, int icon_type,
                                                  int duration_ms);
/* One callback for all four tray events; NULL clears it. */
KUI_C_API KuiStatus kui_window_tray_set_event_callback(KuiWindow* window, KuiTrayEventFn fn,
                                                        void* user_data);
/* Fires the event as if the user did it (automation and tests). */
KUI_C_API KuiStatus kui_window_tray_simulate_event(KuiWindow* window, KuiCTrayEvent event);

/* -- Keyboard shortcuts (window-level) ----------------------------------- */
/* shortcut: "Ctrl+S", "Ctrl+Shift+F5", "Alt+Enter" ... (native ShortcutManager
 * syntax); KUI_ERR_INVALID_ARG if it cannot be parsed. fn runs on the main thread
 * when the window receives the key. Binding a combination that is already bound
 * replaces its callback and yields the same id; the previous user_data is then no
 * longer referenced. */
KUI_C_API KuiStatus kui_window_bind_shortcut(KuiWindow* window, const char* shortcut,
                                              size_t len, KuiTaskFn fn, void* user_data,
                                              uint32_t* out_id);
KUI_C_API KuiStatus kui_window_unbind_shortcut(KuiWindow* window, uint32_t id);
KUI_C_API KuiStatus kui_window_unbind_all_shortcuts(KuiWindow* window);
KUI_C_API KuiStatus kui_window_set_shortcuts_enabled(KuiWindow* window, int enabled);

/* -- Toasts (in-document notifications) ---------------------------------- */
typedef enum KuiCToastLevel {
    KUI_C_TOAST_INFO = 0, KUI_C_TOAST_SUCCESS = 1, KUI_C_TOAST_WARNING = 2,
    KUI_C_TOAST_ERROR = 3
} KuiCToastLevel;
typedef struct KuiCToastOptions {
    const char* message;     /* required */
    size_t message_len;
    const char* title;       /* optional */
    size_t title_len;
    KuiCToastLevel level;
    int duration_ms;         /* 0 = stays until dismissed */
    const char* position;    /* NULL = "bottom-right"; also top-left/-center/-right, bottom-left/-center */
    size_t position_len;
    int closable;
    const char* icon;        /* optional glyph; NULL = by level */
    size_t icon_len;
} KuiCToastOptions;
KUI_C_API KuiStatus kui_document_show_toast(KuiDocument* doc, const KuiCToastOptions* options,
                                             uint32_t* out_id);
KUI_C_API KuiStatus kui_document_dismiss_toast(KuiDocument* doc, uint32_t id);
KUI_C_API KuiStatus kui_document_dismiss_all_toasts(KuiDocument* doc);
KUI_C_API KuiStatus kui_document_get_toast_count(KuiDocument* doc, int* out_count);

/* -- Screenshot ----------------------------------------------------------- */
/* Renders a frame, reads it back and writes it as PNG (the window's current
 * content). out_saved (optional) is 0 when the renderer could not capture or
 * write the file. */
KUI_C_API KuiStatus kui_window_save_screenshot(KuiWindow* window, const char* path,
                                                size_t path_len, int* out_saved);

/* ---------------------------------------------------------------------------
 * 11. Application framework (additive, 3.61.8): router, data binding,
 *     animation state, Mutation / Resize / Intersection observers
 *
 * All of these are main-thread only, callbacks included. Every callback that
 * takes a release_fn (optional, may be NULL) calls it exactly once, when KUI no
 * longer references user_data: after the route / guard / subscription /
 * observer is removed and no longer running, or when the document (for router
 * entries) or the last handle (for observables and observers) goes away.
 * release_fn may run while a document is being destroyed, so it must not call
 * KUI. Element handles inside callback structs are borrowed: valid only during
 * the callback; kui_element_retain one to keep it. When a call fails,
 * release_fn is not called and user_data stays yours.
 * ------------------------------------------------------------------------- */

/* -- Router (one per document) ---------------------------------------------
 * A <kui-router-view> element in the page shows the current route's view (it
 * registers itself while it is in the document); any other container can be
 * registered with kui_document_router_register_view. A view's children are
 * replaced by the current route's view on every navigation.
 * Patterns use ":name" segments ("/users/:id"); "?a=1&b=2" is parsed as query. */
typedef struct KuiCKeyValue {
    const char* key;
    size_t key_len;
    const char* value;
    size_t value_len;
} KuiCKeyValue;
typedef struct KuiCRouteInfo {
    const char* path;      /* full path including the query, e.g. "/users/42?tab=1" */
    size_t path_len;
    const char* pattern;   /* matched pattern, empty when no route matched */
    size_t pattern_len;
    const char* query;     /* text after '?', empty if none */
    size_t query_len;
    const KuiCKeyValue* params;        /* ":name" segments */
    size_t param_count;
    const KuiCKeyValue* query_params;
    size_t query_param_count;
} KuiCRouteInfo;
/* Builds the view for the current route. Return a reference you own (for
 * example from kui_document_create_element); KUI takes it over. NULL leaves the
 * view empty. */
typedef KuiElement* (*KuiRouteViewFn)(KuiDocument* doc, const KuiCRouteInfo* route,
                                      void* user_data);
/* Return nonzero to allow the navigation, 0 to cancel it. Do not add or remove
 * guards from inside a guard. */
typedef int (*KuiRouteGuardFn)(const KuiCRouteInfo* from, const KuiCRouteInfo* to,
                               void* user_data);
typedef void (*KuiRouteInfoFn)(const KuiCRouteInfo* route, void* user_data);
typedef enum KuiCRouteGuardKind {
    KUI_C_ROUTE_GUARD_BEFORE_ENTER = 0, KUI_C_ROUTE_GUARD_BEFORE_LEAVE = 1
} KuiCRouteGuardKind;

/* KUI_ERR_INVALID_ARG for an empty or invalid pattern or one already added. */
KUI_C_API KuiStatus kui_document_router_add_route(KuiDocument* doc, const char* pattern,
                                                   size_t pattern_len, const char* name,
                                                   size_t name_len, KuiRouteViewFn fn,
                                                   void* user_data, KuiTaskFn release_fn);
/* KUI_ERR_NOT_FOUND if no route has this pattern. */
KUI_C_API KuiStatus kui_document_router_remove_route(KuiDocument* doc, const char* pattern,
                                                      size_t len);
KUI_C_API KuiStatus kui_document_router_clear_routes(KuiDocument* doc);
/* el must belong to doc. It is filled right away when a route is current, and
 * kept alive while registered. Loading a new page does not unregister it;
 * register the new page's container again. */
KUI_C_API KuiStatus kui_document_router_register_view(KuiDocument* doc, KuiElement* el);
/* KUI_ERR_NOT_FOUND if el is not registered. */
KUI_C_API KuiStatus kui_document_router_unregister_view(KuiDocument* doc, KuiElement* el);
KUI_C_API KuiStatus kui_document_router_has_route(KuiDocument* doc, const char* pattern,
                                                   size_t len, int* out_has);
KUI_C_API KuiStatus kui_document_router_get_route_count(KuiDocument* doc, uint32_t* out_count);
/* out_navigated (optional) is 0 when a guard cancelled the navigation. A path no
 * route matches still navigates (and empties the view), like the native API. */
KUI_C_API KuiStatus kui_document_router_push(KuiDocument* doc, const char* path, size_t len,
                                              int* out_navigated);
/* Same, replacing the current history entry. */
KUI_C_API KuiStatus kui_document_router_replace(KuiDocument* doc, const char* path, size_t len,
                                                 int* out_navigated);
/* Moves through history: -1 is back, 1 is forward. out_navigated is 0 when the
 * target is outside the history or a guard cancelled. */
KUI_C_API KuiStatus kui_document_router_go(KuiDocument* doc, int delta, int* out_navigated);
/* Calls fn once, synchronously, with the current route. */
KUI_C_API KuiStatus kui_document_router_get_current(KuiDocument* doc, KuiRouteInfoFn fn,
                                                     void* user_data);
KUI_C_API KuiStatus kui_document_router_get_current_path(KuiDocument* doc, char* buf,
                                                          size_t buf_capacity, size_t* out_len);
/* A ":name" parameter of the current route; KUI_ERR_NOT_FOUND if absent. */
KUI_C_API KuiStatus kui_document_router_get_param(KuiDocument* doc, const char* name,
                                                   size_t name_len, char* buf,
                                                   size_t buf_capacity, size_t* out_len);
/* out_index is -1 before the first navigation. Either pointer may be NULL. */
KUI_C_API KuiStatus kui_document_router_get_history(KuiDocument* doc, int* out_index,
                                                     int* out_size);
KUI_C_API KuiStatus kui_document_router_add_guard(KuiDocument* doc, KuiCRouteGuardKind kind,
                                                   KuiRouteGuardFn fn, void* user_data,
                                                   KuiTaskFn release_fn, uint32_t* out_id);
KUI_C_API KuiStatus kui_document_router_remove_guard(KuiDocument* doc, uint32_t id);
/* fn runs after every successful navigation, once the view is updated. */
KUI_C_API KuiStatus kui_document_router_add_navigated_callback(KuiDocument* doc,
                                                                KuiRouteInfoFn fn,
                                                                void* user_data,
                                                                KuiTaskFn release_fn,
                                                                uint32_t* out_id);
KUI_C_API KuiStatus kui_document_router_remove_navigated_callback(KuiDocument* doc,
                                                                   uint32_t id);

/* -- Data binding ------------------------------------------------------------
 * A KuiObservable holds a UTF-8 string value (the native Observable<T> binds
 * through its string form too: "true" / "1" count as true). Binding it to an
 * element property keeps the property in sync: "textContent", "innerHTML",
 * "className", "hidden", "disabled", "value", "checked", "valueAsNumber", or any
 * other name as an attribute. Binding sets the property right away; later
 * changes are batched and applied at the next frame (or
 * kui_document_flush_bindings). A two-way binding also writes the
 * input's "value" / "checked" back into the observable on user input.
 * An observable keeps the elements and documents it is bound to alive until it
 * is unbound; release the last reference on the main thread. */
typedef struct KuiObservable KuiObservable;
/* initial may be NULL (empty). */
KUI_C_API KuiStatus kui_observable_create(const char* initial, size_t len,
                                           KuiObservable** out);
KUI_C_API KuiStatus kui_observable_retain(KuiObservable* obs);
/* Dropping the last reference unbinds it everywhere and releases subscriptions. */
KUI_C_API KuiStatus kui_observable_release(KuiObservable* obs);
/* Notifies subscribers synchronously (only when the value changed). */
KUI_C_API KuiStatus kui_observable_set(KuiObservable* obs, const char* value, size_t len);
KUI_C_API KuiStatus kui_observable_get(KuiObservable* obs, char* buf, size_t buf_capacity,
                                        size_t* out_len);
/* fn runs after every change. */
KUI_C_API KuiStatus kui_observable_subscribe(KuiObservable* obs, KuiTaskFn fn, void* user_data,
                                              KuiTaskFn release_fn, uint32_t* out_id);
KUI_C_API KuiStatus kui_observable_unsubscribe(KuiObservable* obs, uint32_t id);
/* property: see above. KUI_ERR_INVALID_ARG when the element has no document or a
 * two-way binding is not possible for it. */
KUI_C_API KuiStatus kui_observable_bind(KuiObservable* obs, KuiElement* el,
                                         const char* property, size_t property_len,
                                         int two_way, uint32_t* out_binding_id);
/* KUI_ERR_NOT_FOUND if the id is not a live binding of this observable. */
KUI_C_API KuiStatus kui_observable_unbind(KuiObservable* obs, uint32_t binding_id);
KUI_C_API KuiStatus kui_observable_unbind_all(KuiObservable* obs);
/* Applies pending binding updates now instead of at the next frame. */
KUI_C_API KuiStatus kui_document_flush_bindings(KuiDocument* doc);

/* -- Animation state -------------------------------------------------------
 * Control CSS animations and transitions through CSS (class / inline style,
 * animation-play-state); their events ("animationstart", "animationiteration",
 * "animationend", "animationcancel", "transitionrun", "transitionstart",
 * "transitionend", "transitioncancel") reach kui_element_add_event_listener. */
/* Either pointer may be NULL. Nonzero while a CSS animation / transition runs. */
KUI_C_API KuiStatus kui_element_get_animation_state(KuiElement* el, int* out_animating,
                                                     int* out_transitioning);
KUI_C_API KuiStatus kui_document_get_animation_counts(KuiDocument* doc,
                                                       uint32_t* out_animations,
                                                       uint32_t* out_transitions);

/* -- Observers ---------------------------------------------------------------
 * A KuiObserver watches elements of whichever documents they belong to; the
 * targets stay alive while observed. Mutation records arrive synchronously after
 * each change; resize and intersection entries after the layout of a rendered
 * frame (the first observation always reports once). Calling disconnect or
 * release from inside the callback is allowed. Dropping the last reference
 * disconnects the observer; do it on the main thread. */
typedef struct KuiObserver KuiObserver;
typedef enum KuiCMutationType {
    KUI_C_MUTATION_CHILD_LIST = 0, KUI_C_MUTATION_ATTRIBUTES = 1,
    KUI_C_MUTATION_CHARACTER_DATA = 2
} KuiCMutationType;
/* Node fields are elements or NULL; *_node_type gives the DOM nodeType of the
 * underlying node (0 none, 1 element, 3 text, 8 comment), so a text node shows
 * as NULL with type 3. For CHARACTER_DATA the target is the text node's parent
 * element and target_node_type is 3. */
typedef struct KuiCMutationRecord {
    KuiCMutationType type;
    KuiElement* target;
    int target_node_type;
    KuiElement* added;
    int added_node_type;
    KuiElement* removed;
    int removed_node_type;
    KuiElement* previous_sibling;
    KuiElement* next_sibling;
    const char* attribute_name;     /* ATTRIBUTES only */
    size_t attribute_name_len;
    const char* old_value;          /* when requested in the options, else empty */
    size_t old_value_len;
} KuiCMutationRecord;
typedef struct KuiCMutationOptions {
    int child_list;
    int attributes;
    int character_data;
    int subtree;
    int attribute_old_value;
    int character_data_old_value;
    const char* const* attribute_filter;   /* NUL-terminated names; empty = all */
    size_t attribute_filter_count;
} KuiCMutationOptions;
typedef struct KuiCResizeEntry {
    KuiElement* target;
    float content_width;
    float content_height;
    float border_width;
    float border_height;
    float content_x;        /* content box offset inside the border box */
    float content_y;
} KuiCResizeEntry;
typedef struct KuiCIntersectionOptions {
    KuiElement* root;       /* NULL = the viewport */
    float margin_top;       /* grows (or, negative, shrinks) the root bounds */
    float margin_right;
    float margin_bottom;
    float margin_left;
    const float* thresholds;   /* ratios in [0, 1]; none = { 0 } */
    size_t threshold_count;
} KuiCIntersectionOptions;
typedef struct KuiCIntersectionEntry {
    KuiElement* target;
    KuiRect bounding_client_rect;
    KuiRect intersection_rect;
    KuiRect root_bounds;
    float intersection_ratio;
    int is_intersecting;
} KuiCIntersectionEntry;
typedef void (*KuiMutationFn)(const KuiCMutationRecord* records, size_t count, void* user_data);
typedef void (*KuiResizeFn)(const KuiCResizeEntry* entries, size_t count, void* user_data);
typedef void (*KuiIntersectionFn)(const KuiCIntersectionEntry* entries, size_t count,
                                  void* user_data);

KUI_C_API KuiStatus kui_mutation_observer_create(KuiMutationFn fn, void* user_data,
                                                  KuiTaskFn release_fn, KuiObserver** out);
KUI_C_API KuiStatus kui_resize_observer_create(KuiResizeFn fn, void* user_data,
                                                KuiTaskFn release_fn, KuiObserver** out);
/* options NULL = viewport root, no margin, threshold 0. */
KUI_C_API KuiStatus kui_intersection_observer_create(const KuiCIntersectionOptions* options,
                                                      KuiIntersectionFn fn, void* user_data,
                                                      KuiTaskFn release_fn, KuiObserver** out);
/* Mutation observers only. At least one of child_list / attributes /
 * character_data must be set. Observing the same target again replaces its
 * options. */
KUI_C_API KuiStatus kui_mutation_observer_observe(KuiObserver* obs, KuiElement* target,
                                                   const KuiCMutationOptions* options);
/* Resize and intersection observers (a mutation observer needs options:
 * KUI_ERR_INVALID_ARG). */
KUI_C_API KuiStatus kui_observer_observe(KuiObserver* obs, KuiElement* target);
/* Resize and intersection observers; KUI_ERR_NOT_SUPPORTED for mutation
 * observers (the native API has no per-target unobserve for them either). */
KUI_C_API KuiStatus kui_observer_unobserve(KuiObserver* obs, KuiElement* target);
/* Stops observing every target and drops undelivered records. */
KUI_C_API KuiStatus kui_observer_disconnect(KuiObserver* obs);
KUI_C_API KuiStatus kui_observer_retain(KuiObserver* obs);
KUI_C_API KuiStatus kui_observer_release(KuiObserver* obs);

/* ---------------------------------------------------------------------------
 * 12. Popups, input pickers and game audio (additive, 3.61.8)
 *
 * Main-thread only. Popups are the document's tooltip / context-menu layer
 * (drawn over the page, or as native popup windows in native mode); the input
 * picker is the date / time / colour panel of <input>; audio is the
 * document's AudioManager (needs the audio media plugin).
 * ------------------------------------------------------------------------- */

/* -- Popups ------------------------------------------------------------------ */
typedef enum KuiCPopupType {
    KUI_C_POPUP_TOOLTIP = 0, KUI_C_POPUP_CONTEXT_MENU = 1, KUI_C_POPUP_CUSTOM = 2
} KuiCPopupType;
typedef enum KuiCPopupPosition {
    KUI_C_POPUP_TOP = 0, KUI_C_POPUP_BOTTOM = 1, KUI_C_POPUP_LEFT = 2, KUI_C_POPUP_RIGHT = 3
} KuiCPopupPosition;
typedef enum KuiCPopupMode {
    KUI_C_POPUP_MODE_OVERLAY = 0, KUI_C_POPUP_MODE_NATIVE_WINDOW = 1
} KuiCPopupMode;
/* Colours are 0xRRGGBBAA. */
typedef struct KuiCTooltipTheme {
    uint32_t background;
    uint32_t foreground;
    uint32_t border;
    float radius;
} KuiCTooltipTheme;

/* anchor is in viewport coordinates; the tooltip is placed on the given side. */
KUI_C_API KuiStatus kui_document_show_tooltip(KuiDocument* doc, const char* text, size_t len,
                                               const KuiRect* anchor, KuiCPopupPosition position,
                                               uint32_t* out_id);
/* A context menu at (x, y): the items of one menu (KuiCMenuItem from section 10;
 * children and checkable are ignored). Choosing an item calls fn with its id;
 * release_fn follows section 11's rule and runs once the menu is gone. */
KUI_C_API KuiStatus kui_document_show_context_menu(KuiDocument* doc, float x, float y,
                                                    const KuiCMenuItem* items, size_t item_count,
                                                    KuiMenuActivateFn fn, void* user_data,
                                                    KuiTaskFn release_fn, uint32_t* out_id);
/* Unknown ids are ignored. */
KUI_C_API KuiStatus kui_document_dismiss_popup(KuiDocument* doc, uint32_t id);
KUI_C_API KuiStatus kui_document_dismiss_all_popups(KuiDocument* doc);
KUI_C_API KuiStatus kui_document_dismiss_popups_of_type(KuiDocument* doc, KuiCPopupType type);
KUI_C_API KuiStatus kui_document_has_popups(KuiDocument* doc, int* out_any);
KUI_C_API KuiStatus kui_document_has_popup_of_type(KuiDocument* doc, KuiCPopupType type,
                                                    int* out_has);
/* Hover delay before a title="" tooltip appears (default 500 ms). */
KUI_C_API KuiStatus kui_document_set_tooltip_delay(KuiDocument* doc, double ms);
KUI_C_API KuiStatus kui_document_get_tooltip_delay(KuiDocument* doc, double* out_ms);
KUI_C_API KuiStatus kui_document_set_tooltip_theme(KuiDocument* doc, const KuiCTooltipTheme* theme);
KUI_C_API KuiStatus kui_document_get_tooltip_theme(KuiDocument* doc, KuiCTooltipTheme* out_theme);
KUI_C_API KuiStatus kui_document_set_popup_mode(KuiDocument* doc, KuiCPopupMode mode);
/* The built-in default context menu (Cut / Copy / Paste / Select All on inputs, Copy / Select All
 * elsewhere) is OFF by default: right-click and the menu key still dispatch the "contextmenu" event and
 * menus you show yourself are unaffected. Enable it here (for the whole document, kept across
 * kui_document_load_html) or from the page with <html context-menu>. With DevTools' inspector on but the
 * default menu off, right-click shows only "Inspect Element". */
KUI_C_API KuiStatus kui_document_set_default_context_menu_enabled(KuiDocument* doc, int enabled);
KUI_C_API KuiStatus kui_document_get_default_context_menu_enabled(KuiDocument* doc, int* out_enabled);

/* -- Input picker (date / time / colour panel of <input>) ------------------- */
typedef enum KuiCPickerMode {
    KUI_C_PICKER_NONE = 0, KUI_C_PICKER_DATE = 1, KUI_C_PICKER_TIME = 2,
    KUI_C_PICKER_DATETIME_LOCAL = 3, KUI_C_PICKER_COLOR = 4
} KuiCPickerMode;
typedef struct KuiCInputPickerState {
    int open;
    int enabled;
    KuiCPickerMode mode;
    int visible_year;     /* calendar page */
    int visible_month;    /* 1-12 */
    float color_h;        /* 0-360 */
    float color_s;        /* 0-1 */
    float color_v;        /* 0-1 */
    int hour;
    int minute;
} KuiCInputPickerState;
/* el must be an <input>. KUI_C_PICKER_NONE picks the panel from its type;
 * KUI_ERR_NOT_SUPPORTED when that type has no panel or the picker is disabled. */
KUI_C_API KuiStatus kui_element_open_input_picker(KuiElement* el, KuiCPickerMode mode);
KUI_C_API KuiStatus kui_document_close_input_picker(KuiDocument* doc);
/* Writes the panel's value into the input (firing input / change) and closes. */
KUI_C_API KuiStatus kui_document_commit_input_picker(KuiDocument* doc);
KUI_C_API KuiStatus kui_document_get_input_picker_state(KuiDocument* doc,
                                                         KuiCInputPickerState* out_state);
/* The input the picker is open for; KUI_ERR_NOT_FOUND when closed. */
KUI_C_API KuiStatus kui_document_get_input_picker_target(KuiDocument* doc, KuiElement** out);
KUI_C_API KuiStatus kui_document_set_input_picker_enabled(KuiDocument* doc, int enabled);
KUI_C_API KuiStatus kui_document_set_input_picker_month(KuiDocument* doc, int year, int month);
KUI_C_API KuiStatus kui_document_set_input_picker_color_hsv(KuiDocument* doc, float h, float s,
                                                             float v);
KUI_C_API KuiStatus kui_document_set_input_picker_time(KuiDocument* doc, int hour, int minute);

/* -- Game audio (the document's AudioManager) ------------------------------- */
typedef enum KuiCAudioGroup {
    KUI_C_AUDIO_MASTER = 0, KUI_C_AUDIO_MUSIC = 1, KUI_C_AUDIO_SFX = 2, KUI_C_AUDIO_UI = 3
} KuiCAudioGroup;
/* Pass -1 where a function takes "group or all". */
typedef struct KuiCAudioPlayOptions {
    KuiCAudioGroup group;  /* default SFX */
    float volume;          /* 0-1 before group / master scaling */
    float pan;             /* -1 (left) to 1 (right) */
    float pitch;           /* playback speed multiplier, 1 = normal */
    int loop;
    int preload_only;      /* load without playing */
} KuiCAudioPlayOptions;
typedef struct KuiCSoundInfo {
    KuiCAudioGroup group;
    float volume;
    float pan;
    float pitch;
    int playing;
    int looping;
    double current_time;   /* seconds */
    double duration;
} KuiCSoundInfo;
/* out_available is 0 without an audio device or media plugin. */
KUI_C_API KuiStatus kui_document_audio_is_available(KuiDocument* doc, int* out_available);
/* options NULL = SFX, volume 1, pan 0, pitch 1. KUI_ERR_NOT_SUPPORTED when audio
 * is unavailable, KUI_ERR_IO when the file could not be played. */
KUI_C_API KuiStatus kui_document_audio_play(KuiDocument* doc, const char* path, size_t len,
                                             const KuiCAudioPlayOptions* options,
                                             uint32_t* out_handle);
/* Unknown handles are ignored. */
KUI_C_API KuiStatus kui_document_audio_stop(KuiDocument* doc, uint32_t handle);
KUI_C_API KuiStatus kui_document_audio_set_paused(KuiDocument* doc, uint32_t handle, int paused);
KUI_C_API KuiStatus kui_document_audio_set_volume(KuiDocument* doc, uint32_t handle, float volume);
KUI_C_API KuiStatus kui_document_audio_set_pan(KuiDocument* doc, uint32_t handle, float pan);
KUI_C_API KuiStatus kui_document_audio_set_pitch(KuiDocument* doc, uint32_t handle, float pitch);
KUI_C_API KuiStatus kui_document_audio_seek(KuiDocument* doc, uint32_t handle, double seconds);
/* KUI_ERR_NOT_FOUND for an unknown or finished handle. */
KUI_C_API KuiStatus kui_document_audio_get_info(KuiDocument* doc, uint32_t handle,
                                                 KuiCSoundInfo* out_info);
KUI_C_API KuiStatus kui_document_audio_set_group(KuiDocument* doc, KuiCAudioGroup group,
                                                  float volume, int muted);
/* Either pointer may be NULL. */
KUI_C_API KuiStatus kui_document_audio_get_group(KuiDocument* doc, KuiCAudioGroup group,
                                                  float* out_volume, int* out_muted);
/* group -1 = every group. */
KUI_C_API KuiStatus kui_document_audio_stop_all(KuiDocument* doc, int group);
KUI_C_API KuiStatus kui_document_audio_set_all_paused(KuiDocument* doc, int group, int paused);
/* Loads count pooled instances; out_loaded (optional) is how many succeeded. */
KUI_C_API KuiStatus kui_document_audio_preload(KuiDocument* doc, const char* path, size_t len,
                                                uint32_t count, uint32_t* out_loaded);
/* path NULL unloads everything. */
KUI_C_API KuiStatus kui_document_audio_unload(KuiDocument* doc, const char* path, size_t len);
/* Playing sounds in group (-1 = all groups) and the size of the whole pool of
 * preloaded / finished instances. Either pointer may be NULL. */
KUI_C_API KuiStatus kui_document_audio_get_counts(KuiDocument* doc, int group,
                                                   uint32_t* out_active, uint32_t* out_pooled);

/* ---------------------------------------------------------------------------
 * 13. Media and Lottie playback control (additive, 3.61.8)
 *
 * Same element rules as section 8: KUI_ERR_NOT_SUPPORTED when the element is not
 * <audio> / <video> (kui_media_*) or <lottie-player> (kui_lottie_*).
 * ------------------------------------------------------------------------- */
/* Restarts loading the current src (HTMLMediaElement.load()). */
KUI_C_API KuiStatus kui_media_load(KuiElement* element);
KUI_C_API KuiStatus kui_media_set_loop(KuiElement* element, int loop);
KUI_C_API KuiStatus kui_media_set_autoplay(KuiElement* element, int autoplay);
/* Any pointer may be NULL. out_ended is nonzero once playback reached the end
 * without looping. */
KUI_C_API KuiStatus kui_media_get_playback_flags(KuiElement* element, int* out_loop,
                                                  int* out_autoplay, int* out_ended);

typedef enum KuiCLottieLoadState {
    KUI_C_LOTTIE_IDLE = 0, KUI_C_LOTTIE_LOADING = 1, KUI_C_LOTTIE_COMPLETE = 2,
    KUI_C_LOTTIE_ERROR = 3
} KuiCLottieLoadState;
typedef struct KuiCLottieState {
    int playing;
    int looping;
    int autoplay;
    float speed;
    double position;       /* 0-1 through the animation */
    KuiCLottieLoadState load_state;
} KuiCLottieState;
/* Stops and rewinds to the first frame (shown at the next frame). */
KUI_C_API KuiStatus kui_lottie_stop(KuiElement* element);
/* position is 0-1 through the animation; also works while paused. */
KUI_C_API KuiStatus kui_lottie_seek(KuiElement* element, double position);
/* Playback speed multiplier (the speed attribute); 0 or more. */
KUI_C_API KuiStatus kui_lottie_set_speed(KuiElement* element, float speed);
KUI_C_API KuiStatus kui_lottie_set_loop(KuiElement* element, int loop);
KUI_C_API KuiStatus kui_lottie_get_state(KuiElement* element, KuiCLottieState* out_state);

/* ---------------------------------------------------------------------------
 * 14. Extension and DevTools lifecycle (additive, 3.61.8)
 *
 * Extension calls need the document's extension manager: call
 * kui_document_init_extensions first (an empty directory is fine), otherwise
 * they return KUI_ERR_NOT_SUPPORTED. Hot reload watches a directory on a
 * background thread; the reload itself happens on the main thread during the
 * next frame. DevTools: the access token is part of kui_window_get_devtools_url.
 * ------------------------------------------------------------------------- */
/* out_loaded (optional) is 0 when the DLL could not be loaded. */
KUI_C_API KuiStatus kui_document_load_extension(KuiDocument* doc, const char* path, size_t len,
                                                 int* out_loaded);
/* By custom-element tag (e.g. "kui-3d-view"); KUI_ERR_NOT_FOUND if not loaded. */
KUI_C_API KuiStatus kui_document_unload_extension(KuiDocument* doc, const char* tag_name,
                                                   size_t len);
/* Unloads the DLL's extension and loads the file again. */
KUI_C_API KuiStatus kui_document_reload_extension(KuiDocument* doc, const char* path, size_t len,
                                                   int* out_reloaded);
KUI_C_API KuiStatus kui_document_unload_all_extensions(KuiDocument* doc);
/* directory NULL stops watching. */
KUI_C_API KuiStatus kui_document_watch_extensions(KuiDocument* doc, const char* directory,
                                                   size_t len);
/* Either pointer may be NULL. */
KUI_C_API KuiStatus kui_document_get_extension_state(KuiDocument* doc, size_t* out_count,
                                                      int* out_watching);
/* Stops the window's DevTools server; kui_window_start_devtools starts it again. */
KUI_C_API KuiStatus kui_window_stop_devtools(KuiWindow* w);
/* Any pointer may be NULL; port and clients are 0 while stopped. */
KUI_C_API KuiStatus kui_window_get_devtools_state(KuiWindow* w, int* out_running,
                                                   uint16_t* out_port, uint32_t* out_clients);

/* ---------------------------------------------------------------------------
 * 15. Drag-and-drop data (additive, 3.61.8)
 *
 * For drag events ("dragstart", "dragover", "drop", ...; KUI_ERR_NOT_SUPPORTED
 * for other events). An element with draggable="true" starts an in-page drag:
 * set the data in its "dragstart" listener, call kui_event_prevent_default in
 * the target's "dragover" listener to allow the drop, and read the data in
 * "drop". Files dropped from the OS are read with kui_event_drag_get_file*.
 * ------------------------------------------------------------------------- */
typedef enum KuiCDropEffect {
    KUI_C_DROP_NONE = 0, KUI_C_DROP_COPY = 1, KUI_C_DROP_MOVE = 2, KUI_C_DROP_LINK = 3
} KuiCDropEffect;
typedef enum KuiCEffectAllowed {
    KUI_C_EFFECT_NONE = 0, KUI_C_EFFECT_COPY = 1, KUI_C_EFFECT_MOVE = 2, KUI_C_EFFECT_LINK = 3,
    KUI_C_EFFECT_COPY_MOVE = 4, KUI_C_EFFECT_COPY_LINK = 5, KUI_C_EFFECT_MOVE_LINK = 6,
    KUI_C_EFFECT_ALL = 7, KUI_C_EFFECT_UNINITIALIZED = 8
} KuiCEffectAllowed;
/* format is a MIME type such as "text/plain". */
KUI_C_API KuiStatus kui_event_drag_set_data(KuiEvent* event, const char* format, size_t format_len,
                                             const char* data, size_t data_len);
/* KUI_ERR_NOT_FOUND when no data of that format was set. */
KUI_C_API KuiStatus kui_event_drag_get_data(KuiEvent* event, const char* format, size_t format_len,
                                             char* buf, size_t buf_capacity, size_t* out_len);
/* format NULL clears every format. */
KUI_C_API KuiStatus kui_event_drag_clear_data(KuiEvent* event, const char* format, size_t format_len);
/* The formats set so far, in order (DataTransfer.types). */
KUI_C_API KuiStatus kui_event_drag_get_type_count(KuiEvent* event, size_t* out_count);
KUI_C_API KuiStatus kui_event_drag_get_type(KuiEvent* event, size_t index, char* buf,
                                             size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_event_drag_set_drop_effect(KuiEvent* event, KuiCDropEffect effect);
KUI_C_API KuiStatus kui_event_drag_get_drop_effect(KuiEvent* event, KuiCDropEffect* out_effect);
KUI_C_API KuiStatus kui_event_drag_set_effect_allowed(KuiEvent* event, KuiCEffectAllowed effect);
KUI_C_API KuiStatus kui_event_drag_get_effect_allowed(KuiEvent* event,
                                                       KuiCEffectAllowed* out_effect);

/* ---------------------------------------------------------------------------
 * 16. Script engine control (additive, 3.61.8)
 *
 * Values cross the ABI as KuiCScriptValue (native kui::ScriptValue): nil,
 * boolean, integer, float, string, function and light userdata. Lua tables
 * have no ScriptValue form and arrive as nil (same as native).
 *
 * Input values (arguments, globals) are copied by KUI before the call returns,
 * a callback's result right after the callback returns. Output values are
 * borrowed: a result's string and
 * function stay valid until the next kui_script_* call on the same engine, and
 * a callback's arguments only during the callback. kui_script_function_retain
 * keeps a function. Strings carry their length and are NUL-terminated on
 * output; on input string_len 0 means NUL-terminated.
 *
 * Main-thread only, callbacks included. release_fn follows the rule of
 * section 11: called exactly once, when KUI no longer references user_data;
 * not called when the call fails. An engine released inside one of its own
 * callbacks is freed after that callback returns.
 * ------------------------------------------------------------------------- */
typedef struct KuiScriptFunction KuiScriptFunction;
typedef enum KuiCScriptValueType {
    KUI_C_SCRIPT_NIL = 0, KUI_C_SCRIPT_BOOL = 1, KUI_C_SCRIPT_INT = 2, KUI_C_SCRIPT_FLOAT = 3,
    KUI_C_SCRIPT_STRING = 4, KUI_C_SCRIPT_FUNCTION = 5, KUI_C_SCRIPT_USERDATA = 6
} KuiCScriptValueType;
typedef struct KuiCScriptValue {
    KuiCScriptValueType type;
    int bool_value;                    /* KUI_C_SCRIPT_BOOL: 0 or 1 */
    int64_t int_value;                 /* KUI_C_SCRIPT_INT */
    double float_value;                /* KUI_C_SCRIPT_FLOAT */
    const char* string_value;          /* KUI_C_SCRIPT_STRING */
    size_t string_len;
    KuiScriptFunction* function_value; /* KUI_C_SCRIPT_FUNCTION */
    void* userdata;                    /* KUI_C_SCRIPT_USERDATA (Lua light userdata) */
} KuiCScriptValue;
/* A C function callable from Lua. *out_result starts as nil. KUI copies it
 * right after the callback returns, so its string / function must outlive the
 * return (a static buffer, or one owned by user_data, works; KUI does not read
 * it again). Return KUI_OK, or another status to raise a Lua error whose
 * message is out_result's string (a generic message when it is not a string). */
typedef KuiStatus (*KuiScriptFunctionFn)(const KuiCScriptValue* args, size_t arg_count,
                                         KuiCScriptValue* out_result, void* user_data);
typedef void (*KuiScriptPrintFn)(const char* message, size_t message_len, void* user_data);

/* Like kui_script_engine_eval, plus the chunk's first return value. The error
 * text of a failed run is read with kui_script_engine_get_last_error. */
KUI_C_API KuiStatus kui_script_engine_eval_value(KuiScriptEngine* engine, const char* code,
                                                 size_t code_len, const char* source,
                                                 size_t source_len, int* out_success,
                                                 KuiCScriptValue* out_value);
KUI_C_API KuiStatus kui_script_engine_eval_file(KuiScriptEngine* engine, const char* path,
                                                size_t path_len, int* out_success,
                                                KuiCScriptValue* out_value);
/* Calls a global Lua function; *out_success 0 when it is missing or raised. */
KUI_C_API KuiStatus kui_script_engine_call_function(KuiScriptEngine* engine, const char* name,
                                                    size_t name_len, const KuiCScriptValue* args,
                                                    size_t arg_count, int* out_success,
                                                    KuiCScriptValue* out_value);
/* Error text of the last failed run on this engine (eval, eval_value,
 * eval_file, call_function, kui_script_function_call); empty after a
 * successful one. */
KUI_C_API KuiStatus kui_script_engine_get_last_error(KuiScriptEngine* engine, char* buf,
                                                     size_t buf_capacity, size_t* out_len);
/* Sets a global Lua function. release_fn runs once Lua no longer references it
 * (the global was replaced and collected, or the engine went away). */
KUI_C_API KuiStatus kui_script_engine_register_function(KuiScriptEngine* engine,
                                                        const char* name, size_t name_len,
                                                        KuiScriptFunctionFn fn, void* user_data,
                                                        KuiTaskFn release_fn);
KUI_C_API KuiStatus kui_script_engine_set_global(KuiScriptEngine* engine, const char* name,
                                                 size_t name_len, const KuiCScriptValue* value);
KUI_C_API KuiStatus kui_script_engine_get_global(KuiScriptEngine* engine, const char* name,
                                                 size_t name_len, KuiCScriptValue* out_value);
/* Receives Lua print() output (tab-separated, as Lua prints it); NULL restores
 * the default (DevTools console only). Replacing the handler releases the old one. */
KUI_C_API KuiStatus kui_script_engine_set_print_handler(KuiScriptEngine* engine,
                                                        KuiScriptPrintFn fn, void* user_data,
                                                        KuiTaskFn release_fn);
/* 0 lifts the limit. Defaults: 256 MiB and 100 000 000 instructions per run;
 * the instruction limit applies from the next eval. */
KUI_C_API KuiStatus kui_script_engine_set_memory_limit(KuiScriptEngine* engine, size_t bytes);
KUI_C_API KuiStatus kui_script_engine_set_instruction_limit(KuiScriptEngine* engine,
                                                            uint64_t max_instructions);
/* Any pointer may be NULL; a limit of 0 means none. */
KUI_C_API KuiStatus kui_script_engine_get_resource_usage(KuiScriptEngine* engine,
                                                         size_t* out_memory_bytes,
                                                         size_t* out_memory_limit,
                                                         uint64_t* out_instruction_limit);
/* A Lua-callable function that is not a global (pass it as an argument, return
 * it from a callback, or store it with set_global). Owned: release it with
 * kui_script_function_release; release_fn runs once neither the handle nor Lua
 * references it. */
KUI_C_API KuiStatus kui_script_function_create(KuiScriptEngine* engine, KuiScriptFunctionFn fn,
                                               void* user_data, KuiTaskFn release_fn,
                                               KuiScriptFunction** out_function);
/* *out_success 0 when a C function returned an error (message in
 * kui_script_engine_get_last_error). A Lua error inside a Lua function yields
 * nil, as in native C++. KUI_ERR_INVALID_ARG once the engine is gone. */
KUI_C_API KuiStatus kui_script_function_call(KuiScriptFunction* function,
                                             const KuiCScriptValue* args, size_t arg_count,
                                             int* out_success, KuiCScriptValue* out_value);
/* Turns a borrowed (or owned) function into a new owned handle. */
KUI_C_API KuiStatus kui_script_function_retain(KuiScriptFunction* function,
                                               KuiScriptFunction** out_owned);
/* Owned handles only (KUI_ERR_INVALID_ARG for a borrowed one). */
KUI_C_API KuiStatus kui_script_function_release(KuiScriptFunction* function);

/* ---------------------------------------------------------------------------
 * 17. Custom element paint, tick and input (additive, 3.61.8)
 *
 * kui_document_define_custom_element_ex defines a custom element with the
 * lifecycle callbacks of KuiCCustomElementCallbacks plus custom painting,
 * a tick and keyboard / mouse / wheel / IME callbacks (native
 * CustomElementDefinition). All callbacks share callbacks.user_data;
 * release_fn (may be NULL) is called once when the definition goes away with
 * its document, so it must not call KUI. Main-thread only.
 *
 * Input callbacks return 1 when they consumed the event (KUI then skips its
 * default handling), 0 to let it continue. Keyboard and IME callbacks reach
 * the focused element (tabindex + kui_element_focus); mouse and wheel
 * coordinates are in the element's content box. modifiers is a
 * KuiCInputModifier mask -- note it differs from the DevTools injection bits.
 *
 * The paint callback draws into a KuiPaintContext valid only during the call,
 * in content-box CSS pixels. Colours are 0xRRGGBBAA. The callback runs inside
 * a save / restore pair, so its transforms, clips and opacity end with it (a
 * save left open is restored too). Request repaints with
 * kui_element_request_repaint / kui_element_invalidate_rect after state that
 * the painting depends on changes. Native intrinsic-size callbacks are not
 * offered: KUI layout does not consult them yet.
 * ------------------------------------------------------------------------- */
typedef struct KuiPaintContext KuiPaintContext;
typedef enum KuiCInputModifier {
    KUI_C_INPUT_MOD_CTRL = 1, KUI_C_INPUT_MOD_SHIFT = 2, KUI_C_INPUT_MOD_ALT = 4,
    KUI_C_INPUT_MOD_META = 8
} KuiCInputModifier;
typedef enum KuiCCustomKeyType { KUI_C_CUSTOM_KEY_DOWN = 0, KUI_C_CUSTOM_KEY_UP = 1 } KuiCCustomKeyType;
typedef enum KuiCCustomMouseType {
    KUI_C_CUSTOM_MOUSE_DOWN = 0, KUI_C_CUSTOM_MOUSE_UP = 1, KUI_C_CUSTOM_MOUSE_MOVE = 2,
    KUI_C_CUSTOM_MOUSE_ENTER = 3, KUI_C_CUSTOM_MOUSE_LEAVE = 4
} KuiCCustomMouseType;
typedef enum KuiCImeEventType {
    KUI_C_IME_START = 0, KUI_C_IME_UPDATE = 1, KUI_C_IME_COMMIT = 2, KUI_C_IME_CANCEL = 3
} KuiCImeEventType;
typedef enum KuiCDecorationStyle {
    KUI_C_DECORATION_SOLID = 0, KUI_C_DECORATION_DOUBLE = 1, KUI_C_DECORATION_DOTTED = 2,
    KUI_C_DECORATION_DASHED = 3, KUI_C_DECORATION_WAVY = 4
} KuiCDecorationStyle;
typedef struct KuiCCustomKeyEvent {
    KuiCCustomKeyType type;
    const char* key;      /* "a", "Enter", ... */
    size_t key_len;
    const char* code;     /* "KeyA", "Enter", ... */
    size_t code_len;
    int modifiers;
    int is_repeat;
} KuiCCustomKeyEvent;
typedef struct KuiCCustomMouseEvent {
    KuiCCustomMouseType type;
    float x, y;           /* content-box CSS pixels */
    int button;           /* 0 left, 1 middle, 2 right; -1 for move / enter / leave */
    int buttons;          /* pressed buttons */
    int modifiers;
    int click_count;
} KuiCCustomMouseEvent;
typedef struct KuiCCustomWheelEvent {
    float x, y;
    float delta_x, delta_y;
    int modifiers;
} KuiCCustomWheelEvent;
typedef struct KuiCCustomImeEvent {
    KuiCImeEventType type;
    const char* text;     /* composition text (UPDATE) */
    size_t text_len;
    int caret_pos;        /* byte offset in text (UPDATE) */
    const char* committed_text;  /* COMMIT */
    size_t committed_len;
} KuiCCustomImeEvent;
typedef void (*KuiCustomElementPaintFn)(KuiElement* element, KuiPaintContext* paint, void* user_data);
/* delta_seconds since the previous tick. A connected element ticks about every
 * 250 ms; a tick that requests a repaint (kui_element_request_repaint /
 * kui_element_invalidate_rect) is followed by a tick on the next frame. */
typedef void (*KuiCustomElementTickFn)(KuiElement* element, double delta_seconds, void* user_data);
typedef int (*KuiCustomElementNeedsTickFn)(KuiElement* element, void* user_data);
typedef int (*KuiCustomElementKeyFn)(KuiElement* element, const KuiCCustomKeyEvent* event,
                                     void* user_data);
typedef int (*KuiCustomElementMouseFn)(KuiElement* element, const KuiCCustomMouseEvent* event,
                                       void* user_data);
typedef int (*KuiCustomElementWheelFn)(KuiElement* element, const KuiCCustomWheelEvent* event,
                                       void* user_data);
typedef int (*KuiCustomElementImeFn)(KuiElement* element, const KuiCCustomImeEvent* event,
                                     void* user_data);
typedef struct KuiCCustomElementDefinition {
    KuiCCustomElementCallbacks callbacks;  /* lifecycle, observed attributes, user_data */
    KuiCustomElementPaintFn paint;
    KuiCustomElementTickFn tick;
    KuiCustomElementNeedsTickFn needs_tick;  /* NULL = tick every frame while connected */
    KuiCustomElementKeyFn key;
    KuiCustomElementMouseFn mouse;
    KuiCustomElementWheelFn wheel;
    KuiCustomElementImeFn ime;
    KuiTaskFn release_fn;  /* releases callbacks.user_data */
} KuiCCustomElementDefinition;
typedef struct KuiCTextStyle {
    const char* font_family;  /* CSS font-family list, NUL-terminated; NULL = sans-serif */
    float font_size;          /* px; 0 = 14 */
    uint16_t font_weight;     /* 100..900; 0 = 400 */
    int italic;
    float letter_spacing;
    float word_spacing;
} KuiCTextStyle;
/* Elliptical corners (horizontal, vertical radius each). */
typedef struct KuiCCornerRadii {
    float top_left_x, top_left_y, top_right_x, top_right_y;
    float bottom_right_x, bottom_right_y, bottom_left_x, bottom_left_y;
} KuiCCornerRadii;
typedef struct KuiCTextMetrics {
    float width;              /* advance width */
    float ascent, descent;    /* positive distances from the baseline */
    float line_height;        /* CSS "normal" line height */
} KuiCTextMetrics;

/* KUI_ERR_INVALID_ARG for a malformed tag or definition, or a tag already defined. */
KUI_C_API KuiStatus kui_document_define_custom_element_ex(
    KuiDocument* document, const char* tag_name, size_t len,
    const KuiCCustomElementDefinition* definition);
KUI_C_API KuiStatus kui_paint_get_info(KuiPaintContext* paint, float* out_width,
                                       float* out_height, float* out_device_pixel_ratio);
KUI_C_API KuiStatus kui_paint_fill_rect(KuiPaintContext* paint, float x, float y, float width,
                                        float height, uint32_t rgba);
KUI_C_API KuiStatus kui_paint_fill_rounded_rect(KuiPaintContext* paint, float x, float y,
                                                float width, float height,
                                                const KuiCCornerRadii* radii, uint32_t rgba);
KUI_C_API KuiStatus kui_paint_stroke_rect(KuiPaintContext* paint, float x, float y, float width,
                                          float height, float line_width, uint32_t rgba);
KUI_C_API KuiStatus kui_paint_stroke_rounded_rect(KuiPaintContext* paint, float x, float y,
                                                  float width, float height,
                                                  const KuiCCornerRadii* radii, float line_width,
                                                  uint32_t rgba);
/* (x, baseline_y) is the top-left of the text's line box, NOT the baseline: the glyphs hang below it
 * and the baseline lies at y + ascent (ascent from kui_element_measure_text for the same style).
 * The parameter keeps its historical name, which is part of the frozen signature digest.
 * style NULL = sans-serif 14px. */
KUI_C_API KuiStatus kui_paint_draw_text(KuiPaintContext* paint, const char* text, size_t len,
                                        float x, float baseline_y, const KuiCTextStyle* style,
                                        uint32_t rgba);
/* pixels: width * height premultiplied 0xAARRGGBB words, scaled into the rectangle. */
KUI_C_API KuiStatus kui_paint_draw_pixels(KuiPaintContext* paint, const uint32_t* pixels,
                                          uint32_t width, uint32_t height, float x, float y,
                                          float dest_width, float dest_height);
/* An image file (same loading and cache as <img>); src in image pixels. */
KUI_C_API KuiStatus kui_paint_draw_image(KuiPaintContext* paint, const char* path, size_t len,
                                         const KuiRect* src, const KuiRect* dest);
KUI_C_API KuiStatus kui_paint_draw_decoration_line(KuiPaintContext* paint, float x, float y,
                                                   float width, float thickness,
                                                   KuiCDecorationStyle style, uint32_t rgba);
/* restore without a matching save: KUI_ERR_INVALID_ARG. */
KUI_C_API KuiStatus kui_paint_save(KuiPaintContext* paint);
KUI_C_API KuiStatus kui_paint_restore(KuiPaintContext* paint);
KUI_C_API KuiStatus kui_paint_translate(KuiPaintContext* paint, float dx, float dy);
KUI_C_API KuiStatus kui_paint_scale(KuiPaintContext* paint, float sx, float sy);
KUI_C_API KuiStatus kui_paint_rotate(KuiPaintContext* paint, float degrees);
/* Multiplies the opacity of what is drawn next (0..1), until the matching restore. */
KUI_C_API KuiStatus kui_paint_set_opacity(KuiPaintContext* paint, float opacity);
KUI_C_API KuiStatus kui_paint_clip_rect(KuiPaintContext* paint, float x, float y, float width,
                                        float height);
KUI_C_API KuiStatus kui_paint_clip_rounded_rect(KuiPaintContext* paint, float x, float y,
                                                float width, float height,
                                                const KuiCCornerRadii* radii);
/* Same shaping as KUI's own text. KUI_ERR_NOT_SUPPORTED before the element's
 * document has a renderer (no window yet). */
KUI_C_API KuiStatus kui_element_measure_text(KuiElement* element, const char* text, size_t len,
                                             const KuiCTextStyle* style,
                                             KuiCTextMetrics* out_metrics);
/* Repaints the element on the next frame. */
KUI_C_API KuiStatus kui_element_request_repaint(KuiElement* element);
/* Repaints only this content-box rectangle (whole document before the first layout). */
KUI_C_API KuiStatus kui_element_invalidate_rect(KuiElement* element, float x, float y,
                                                float width, float height);
/* Injects an IME composition step at the focused element, as the platform IME
 * would: START, UPDATE (text = composition), COMMIT (text = final text), CANCEL. */
KUI_C_API KuiStatus kui_devtools_dispatch_ime_event(KuiDocument* document, KuiCImeEventType type,
                                                    const char* text, size_t len);

/* ---------------------------------------------------------------------------
 * 18. Components (additive, 3.61.8): built-in custom elements, themes,
 *     VirtualList, ListView, TreeView
 *
 * The native C++ components, main-thread only, callbacks included. Each
 * component handle owns its component; releasing it removes the component's
 * listeners from the container (the rendered rows stay until the container is
 * cleared). A handle released inside one of its own callbacks is freed after
 * that callback returns. release_fn follows the rule of section 11.
 * ------------------------------------------------------------------------- */
/* <kui-dialog>, <kui-tabs>, <kui-tooltip>, <kui-badge>, <kui-tree>, <kui-table>,
 * <kui-menu> and friends. */
KUI_C_API KuiStatus kui_document_register_builtin_components(KuiDocument* document);

/* -- Themes: named sets of CSS custom properties ("light" and "dark" built in) */
typedef struct KuiThemeManager KuiThemeManager;
KUI_C_API KuiStatus kui_theme_manager_create(KuiThemeManager** out_manager);
KUI_C_API KuiStatus kui_theme_manager_release(KuiThemeManager* manager);
/* Adds or replaces a theme; each variable is a custom property ("--bg" = "#fff"). */
KUI_C_API KuiStatus kui_theme_manager_register(KuiThemeManager* manager, const char* name,
                                               size_t name_len, const KuiCKeyValue* variables,
                                               size_t variable_count);
/* Sets the theme's variables as inline custom properties on <html>.
 * KUI_ERR_NOT_FOUND for an unknown theme. */
KUI_C_API KuiStatus kui_theme_manager_apply(KuiThemeManager* manager, const char* name,
                                            size_t name_len, KuiDocument* document);
/* The last theme applied; empty before the first. */
KUI_C_API KuiStatus kui_theme_manager_get_active(KuiThemeManager* manager, char* buf,
                                                 size_t buf_capacity, size_t* out_len);
KUI_C_API KuiStatus kui_theme_manager_get_theme_count(KuiThemeManager* manager,
                                                      size_t* out_count);
KUI_C_API KuiStatus kui_theme_manager_get_theme_name(KuiThemeManager* manager, size_t index,
                                                     char* buf, size_t buf_capacity,
                                                     size_t* out_len);
/* A theme's variables, in no particular order. KUI_ERR_NOT_FOUND for an
 * unknown theme or index. */
KUI_C_API KuiStatus kui_theme_manager_get_variable_count(KuiThemeManager* manager,
                                                         const char* theme, size_t theme_len,
                                                         size_t* out_count);
KUI_C_API KuiStatus kui_theme_manager_get_variable(KuiThemeManager* manager, const char* theme,
                                                   size_t theme_len, size_t index, char* name_buf,
                                                   size_t name_capacity, size_t* out_name_len,
                                                   char* value_buf, size_t value_capacity,
                                                   size_t* out_value_len);

/* -- VirtualList: only the rows in view (plus overscan) are in the DOM ----- */
typedef struct KuiVirtualList KuiVirtualList;
typedef enum KuiCScrollUnit { KUI_C_SCROLL_PIXEL = 0, KUI_C_SCROLL_LINE = 1 } KuiCScrollUnit;
/* Returns a new element for row index (kui_document_create_element, or
 * kui_element_retain on an element you keep); KUI takes that reference. */
typedef KuiElement* (*KuiVirtualListRenderFn)(KuiDocument* document, uint32_t index,
                                              void* user_data);
typedef float (*KuiVirtualListHeightFn)(uint32_t index, void* user_data);
typedef void (*KuiVirtualListFocusFn)(int32_t old_index, int32_t new_index, void* user_data);
typedef void (*KuiIndexFn)(uint32_t index, void* user_data);
typedef struct KuiCVirtualListConfig {
    uint32_t total_items;
    float item_height;                      /* fixed row height (px); 0 = 32 */
    uint32_t overscan;                      /* extra rows above / below the viewport */
    uint32_t recycle_pool_max;              /* 0 = automatic */
    KuiCScrollUnit scroll_unit;
    int enable_keyboard;                    /* arrows / Home / End / PageUp / PageDown / Enter */
    KuiVirtualListRenderFn render_item;     /* required */
    KuiVirtualListHeightFn height_for_item; /* NULL = item_height for every row */
    KuiVirtualListFocusFn on_focus_change;  /* keyboard focus moved */
    KuiIndexFn on_activate;                 /* Enter / Space on the focused row */
    void* user_data;
    KuiTaskFn release_fn;
} KuiCVirtualListConfig;
typedef struct KuiCVirtualListState {
    uint32_t visible_start, visible_end;    /* rendered rows [start, end) */
    uint32_t total_items;
    uint32_t recycle_pool_size;
    int32_t focused_index;                  /* -1 = none */
    float total_height;
    float item_height;
    KuiCScrollUnit scroll_unit;
} KuiCVirtualListState;
/* Sets the container up for virtual scrolling (overflow, spacer) and renders
 * the first rows before returning. */
KUI_C_API KuiStatus kui_virtual_list_create(KuiElement* container,
                                            const KuiCVirtualListConfig* config,
                                            KuiVirtualList** out_list);
KUI_C_API KuiStatus kui_virtual_list_release(KuiVirtualList* list);
KUI_C_API KuiStatus kui_virtual_list_get_state(KuiVirtualList* list,
                                               KuiCVirtualListState* out_state);
KUI_C_API KuiStatus kui_virtual_list_set_total_items(KuiVirtualList* list, uint32_t count);
KUI_C_API KuiStatus kui_virtual_list_insert_items(KuiVirtualList* list, uint32_t index,
                                                  uint32_t count);
KUI_C_API KuiStatus kui_virtual_list_remove_items(KuiVirtualList* list, uint32_t index,
                                                  uint32_t count);
/* Re-renders the rows in view (after their data changed). */
KUI_C_API KuiStatus kui_virtual_list_refresh(KuiVirtualList* list);
KUI_C_API KuiStatus kui_virtual_list_scroll_to_index(KuiVirtualList* list, uint32_t index);
KUI_C_API KuiStatus kui_virtual_list_scroll_to_top(KuiVirtualList* list);
KUI_C_API KuiStatus kui_virtual_list_scroll_to_bottom(KuiVirtualList* list);
KUI_C_API KuiStatus kui_virtual_list_set_scroll_unit(KuiVirtualList* list, KuiCScrollUnit unit);
/* Clamped to the rows; -1 clears. Calls on_focus_change when it changes. */
KUI_C_API KuiStatus kui_virtual_list_set_focused_index(KuiVirtualList* list, int32_t index);
KUI_C_API KuiStatus kui_virtual_list_move_focus(KuiVirtualList* list, int32_t delta);

/* -- ListView: a selectable list on a VirtualList --------------------------- */
typedef struct KuiListView KuiListView;
typedef enum KuiCSelectionMode {
    KUI_C_SELECTION_NONE = 0, KUI_C_SELECTION_SINGLE = 1, KUI_C_SELECTION_MULTI = 2
} KuiCSelectionMode;
typedef struct KuiCListViewItem {
    const char* text;
    size_t text_len;
    const char* subtitle;   /* optional second line */
    size_t subtitle_len;
    const char* icon;       /* optional icon text (emoji or a character) */
    size_t icon_len;
    int disabled;
} KuiCListViewItem;
/* Fills *out_item (zeroed). KUI copies it right after the callback returns, so
 * the strings must outlive the return. */
typedef void (*KuiListViewItemFn)(uint32_t index, KuiCListViewItem* out_item, void* user_data);
typedef struct KuiCListViewConfig {
    uint32_t item_count;
    float item_height;               /* 0 = 36 */
    uint32_t overscan;
    KuiCScrollUnit scroll_unit;
    KuiCSelectionMode selection_mode;
    /* CSS colours, NUL-terminated; NULL keeps the default (dark theme). */
    const char* item_bg;
    const char* item_hover_bg;
    const char* item_selected_bg;
    const char* item_text_color;
    const char* item_sub_color;
    KuiListViewItemFn item_provider; /* required */
    KuiIndexFn on_select;            /* click / keyboard selection */
    KuiIndexFn on_activate;          /* double click / Enter */
    void* user_data;
    KuiTaskFn release_fn;
} KuiCListViewConfig;
KUI_C_API KuiStatus kui_list_view_create(KuiElement* container, const KuiCListViewConfig* config,
                                         KuiListView** out_list);
KUI_C_API KuiStatus kui_list_view_release(KuiListView* list);
KUI_C_API KuiStatus kui_list_view_select(KuiListView* list, uint32_t index);
KUI_C_API KuiStatus kui_list_view_deselect(KuiListView* list, uint32_t index);
KUI_C_API KuiStatus kui_list_view_clear_selection(KuiListView* list);
/* Adds rows [start, end) to the selection. */
KUI_C_API KuiStatus kui_list_view_select_range(KuiListView* list, uint32_t start, uint32_t end);
KUI_C_API KuiStatus kui_list_view_is_selected(KuiListView* list, uint32_t index, int* out_selected);
/* Selected indices in ascending order (two-call: buf NULL for the count). */
KUI_C_API KuiStatus kui_list_view_get_selected(KuiListView* list, uint32_t* buf,
                                               size_t buf_capacity, size_t* out_count);
KUI_C_API KuiStatus kui_list_view_get_focused_index(KuiListView* list, int32_t* out_index);
KUI_C_API KuiStatus kui_list_view_set_item_count(KuiListView* list, uint32_t count);
KUI_C_API KuiStatus kui_list_view_refresh(KuiListView* list);
KUI_C_API KuiStatus kui_list_view_scroll_to_index(KuiListView* list, uint32_t index);
KUI_C_API KuiStatus kui_list_view_scroll_to_top(KuiListView* list);
KUI_C_API KuiStatus kui_list_view_scroll_to_bottom(KuiListView* list);
KUI_C_API KuiStatus kui_list_view_set_scroll_unit(KuiListView* list, KuiCScrollUnit unit);
/* The state of the list's VirtualList. */
KUI_C_API KuiStatus kui_list_view_get_state(KuiListView* list, KuiCVirtualListState* out_state);

/* -- TreeView: an expandable tree on a VirtualList -------------------------- */
typedef struct KuiTreeView KuiTreeView;
typedef void (*KuiTreeToggleFn)(uint32_t node, int expanded, void* user_data);
/* selected: ascending node indices, valid during the callback. */
typedef void (*KuiTreeSelectionFn)(const uint32_t* selected, size_t count, void* user_data);
typedef struct KuiCTreeViewConfig {
    float item_height;               /* 0 = 28 */
    float indent_width;              /* 0 = 20 */
    uint32_t overscan;
    KuiCScrollUnit scroll_unit;
    const char* item_bg;             /* CSS colours; NULL keeps the default */
    const char* item_hover_bg;
    const char* item_selected_bg;
    const char* item_text_color;
    int multi_select;                /* Ctrl+click toggles, Shift+click selects a range */
    KuiIndexFn on_select;
    KuiTreeToggleFn on_toggle;
    KuiTreeSelectionFn on_selection_changed;  /* multi_select only */
    void* user_data;
    KuiTaskFn release_fn;
} KuiCTreeViewConfig;
/* Strings are borrowed until the next call on this tree. */
typedef struct KuiCTreeNode {
    const char* text;
    size_t text_len;
    const char* icon;
    size_t icon_len;
    int expanded;
    int disabled;
    int32_t parent;                  /* -1 = root level */
    size_t child_count;
} KuiCTreeNode;
/* An empty tree: add nodes, then kui_tree_view_setup (nodes added later appear
 * after kui_tree_view_refresh). */
KUI_C_API KuiStatus kui_tree_view_create(KuiTreeView** out_tree);
KUI_C_API KuiStatus kui_tree_view_release(KuiTreeView* tree);
KUI_C_API KuiStatus kui_tree_view_add_node(KuiTreeView* tree, const char* text, size_t text_len,
                                           const char* icon, size_t icon_len, uint32_t* out_index);
KUI_C_API KuiStatus kui_tree_view_add_child(KuiTreeView* tree, uint32_t parent, const char* text,
                                            size_t text_len, const char* icon, size_t icon_len,
                                            uint32_t* out_index);
KUI_C_API KuiStatus kui_tree_view_get_node_count(KuiTreeView* tree, uint32_t* out_count);
KUI_C_API KuiStatus kui_tree_view_get_node(KuiTreeView* tree, uint32_t index, KuiCTreeNode* out_node);
KUI_C_API KuiStatus kui_tree_view_get_node_child(KuiTreeView* tree, uint32_t index,
                                                 size_t position, uint32_t* out_child);
/* text / icon NULL keep the current value; refresh to show the change. */
KUI_C_API KuiStatus kui_tree_view_set_node(KuiTreeView* tree, uint32_t index, const char* text,
                                           size_t text_len, const char* icon, size_t icon_len,
                                           int disabled);
KUI_C_API KuiStatus kui_tree_view_setup(KuiTreeView* tree, KuiElement* container,
                                        const KuiCTreeViewConfig* config);
KUI_C_API KuiStatus kui_tree_view_expand(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_collapse(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_toggle(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_expand_all(KuiTreeView* tree);
KUI_C_API KuiStatus kui_tree_view_collapse_all(KuiTreeView* tree);
KUI_C_API KuiStatus kui_tree_view_select(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_clear_selection(KuiTreeView* tree);
/* Multi-select: Ctrl+click toggles one node, Shift+click selects from the anchor. */
KUI_C_API KuiStatus kui_tree_view_toggle_select(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_select_range(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_select_all(KuiTreeView* tree);
KUI_C_API KuiStatus kui_tree_view_is_selected(KuiTreeView* tree, uint32_t index, int* out_selected);
/* -1 when nothing is selected. */
KUI_C_API KuiStatus kui_tree_view_get_selected_index(KuiTreeView* tree, int32_t* out_index);
/* The same answer as is_selected, in ascending order: the selected set of a
 * multi-select tree, the selected node otherwise (two-call: buf NULL for the count). */
KUI_C_API KuiStatus kui_tree_view_get_selected(KuiTreeView* tree, uint32_t* buf,
                                               size_t buf_capacity, size_t* out_count);
KUI_C_API KuiStatus kui_tree_view_refresh(KuiTreeView* tree);
KUI_C_API KuiStatus kui_tree_view_scroll_to_node(KuiTreeView* tree, uint32_t index);
KUI_C_API KuiStatus kui_tree_view_set_scroll_unit(KuiTreeView* tree, KuiCScrollUnit unit);
/* Rows currently shown (expanded nodes and their visible descendants). */
KUI_C_API KuiStatus kui_tree_view_get_visible_row_count(KuiTreeView* tree, uint32_t* out_count);

/* ---------------------------------------------------------------------------
 * 19. Document style profile (additive, 3.61.8)
 *
 * NATIVE_GUI (the default) keeps KUI's GUI defaults; BROWSER_COMPAT uses the
 * browser's document defaults (the default font is serif), as a page written
 * for browsers expects. Author styles override either. Set it before loading
 * the page; changing it reloads the user-agent stylesheet.
 * ------------------------------------------------------------------------- */
typedef enum KuiCStyleProfile {
    KUI_C_STYLE_NATIVE_GUI = 0, KUI_C_STYLE_BROWSER_COMPAT = 1
} KuiCStyleProfile;
KUI_C_API KuiStatus kui_document_set_style_profile(KuiDocument* document, KuiCStyleProfile profile);
KUI_C_API KuiStatus kui_document_get_style_profile(KuiDocument* document,
                                                   KuiCStyleProfile* out_profile);

/* ---------------------------------------------------------------------------
 * Host integration additions (appended in 3.x, nothing above changes).
 * ------------------------------------------------------------------------- */

/* Starts an inertial scroll of the element, velocity in px/ms (positive y scrolls down). */
KUI_C_API KuiStatus kui_element_start_momentum_scroll(KuiElement* el, float velocity_x,
                                                      float velocity_y);
/* Scrolls the nearest scrollable ancestors so the element is visible; align_to_top != 0 aligns
 * its top edge with the container's top, 0 aligns the bottom edge. */
KUI_C_API KuiStatus kui_element_scroll_into_view(KuiElement* el, int align_to_top);

/* The wheel delta of a "wheel" event (the event must be a WheelEvent, else KUI_ERR_NOT_SUPPORTED).
 * Either pointer may be NULL. */
KUI_C_API KuiStatus kui_event_wheel_get_delta(KuiEvent* evt, float* out_delta_x, float* out_delta_y);

/* Runs the Lua engine's due timers (setTimeout / setInterval) and sets the engine's clock to `now_ms`
 * (milliseconds; pass kui_platform_now each frame). A timer is due `delay` after the engine's clock
 * as it was when the script scheduled it -- 0 before the first tick -- so timers scheduled by a script
 * that ran before the first tick fire on that first tick. The engine does not tick itself: a host
 * that loads scripts with timers calls this once per frame. KUI_ERR_NOT_SUPPORTED when scripting is
 * not built in. */
KUI_C_API KuiStatus kui_script_engine_tick_timers(KuiScriptEngine* engine, double now_ms);

/* The directory relative resource paths (images, stylesheets, scripts) of the document resolve
 * against; end it with a path separator. set_base_path does not reload anything. */
KUI_C_API KuiStatus kui_document_set_base_path(KuiDocument* doc, const char* path, size_t path_len);
KUI_C_API KuiStatus kui_document_get_base_path(KuiDocument* doc, char* buf, size_t buf_capacity,
                                               size_t* out_len);
/* Sets the render backend's device pixel ratio (the scale the page is rasterized at).
 * KUI_ERR_NOT_SUPPORTED before the document has a renderer (no window yet). */
KUI_C_API KuiStatus kui_document_set_render_device_pixel_ratio(KuiDocument* doc, float dpr);

/* kui_window_create with the options KuiWindowProps does not have. struct_size must be
 * sizeof(KuiWindowPropsEx) (later versions append fields and accept the smaller struct). Zero
 * fields keep the library default, like KuiWindowProps. */
typedef struct KuiWindowPropsEx {
    size_t struct_size;
    KuiWindowProps base;     /* everything kui_window_create takes */
    int    sizeable;         /* frameless windows: resize by dragging the edges */
    int    hidden;           /* create without showing; call kui_window_show later */
    int    has_frame_radius; /* 1 = frame_radius is used, 0 = the platform default */
    float  frame_radius;     /* corner radius of the top-level window, CSS px / DIP */
} KuiWindowPropsEx;
KUI_C_API KuiStatus kui_window_create_ex(const KuiWindowPropsEx* props, KuiWindow** out);

/* Enables or disables mouse and keyboard input to the window (a modal parent). Windows only; other
 * platforms return KUI_ERR_NOT_SUPPORTED. */
KUI_C_API KuiStatus kui_window_set_enabled(KuiWindow* w, int enabled);
KUI_C_API KuiStatus kui_window_is_enabled(KuiWindow* w, int* out_enabled);
/* Handles pending events, updates the document and paints one frame now (what the platform loop
 * does each frame). Typical use: render the first frame while the window is still hidden. */
KUI_C_API KuiStatus kui_window_process_frame(KuiWindow* w);
/* Turns the DevTools WebSocket token check on or off. The window's DevTools server must be
 * running (kui_window_start_devtools), else KUI_ERR_NOT_FOUND. With the check off, any local
 * client may connect -- only for tools that cannot pass the token. */
KUI_C_API KuiStatus kui_window_set_devtools_token_auth(KuiWindow* w, int enabled);

/* Turns the operating system's input method (IME) off for the calling thread, so letter and space
 * keys are never taken by a Chinese / Japanese / Korean input method. Windows only; other
 * platforms return KUI_ERR_NOT_SUPPORTED. */
KUI_C_API KuiStatus kui_platform_disable_ime(void);

/* The in-process console (log messages shown by DevTools). */
typedef enum KuiCLogLevel {
    KUI_C_LOG_DEBUG = 0, KUI_C_LOG_INFO = 1, KUI_C_LOG_WARN = 2, KUI_C_LOG_ERROR = 3
} KuiCLogLevel;
KUI_C_API KuiStatus kui_console_set_enabled(int enabled);
/* Prints every message to stderr as it arrives. */
KUI_C_API KuiStatus kui_console_set_stderr_output(int enabled);
/* Messages below this level are dropped. */
KUI_C_API KuiStatus kui_console_set_min_level(KuiCLogLevel level);

/* Crash diagnostics. Install once near the top of main, before starting threads. */
enum { KUI_C_STACK_TRACE_FILE = 1, KUI_C_STACK_TRACE_STDERR = 2, KUI_C_STACK_TRACE_MESSAGE_BOX = 4 };
typedef struct KuiCStackTraceConfig {
    size_t      struct_size;     /* sizeof(KuiCStackTraceConfig) */
    unsigned    output_mode;     /* KUI_C_STACK_TRACE_* bits; 0 = FILE | STDERR */
    const char* log_file_path;   /* NULL / "" = the library default */
    int         max_frames;      /* 0 = 64 */
    const char* app_name;        /* NULL = "kui_app" */
    int         write_mini_dump; /* Windows: also write a .dmp */
    const char* dump_dir;        /* NULL / "" = the library default */
} KuiCStackTraceConfig;
/* Installs the unhandled-exception / signal stack trace writer. config NULL = defaults.
 * *out_installed (optional) is 1 when installed. */
KUI_C_API KuiStatus kui_stack_trace_install(const KuiCStackTraceConfig* config, int* out_installed);
KUI_C_API KuiStatus kui_stack_trace_uninstall(void);
typedef struct KuiCCrashReporterConfig {
    size_t      struct_size;     /* sizeof(KuiCCrashReporterConfig) */
    const char* handler_exe;     /* NULL / "" = crashpad_handler next to the executable */
    const char* dump_dir;        /* NULL / "" = <exe dir>/crashes */
    const char* app_name;        /* NULL = "kui_app" */
    const char* app_version;     /* NULL = "0.0.0" */
    const char* report_url;      /* NULL / "" = never upload */
    int         enable_upload;
} KuiCCrashReporterConfig;
/* Installs the Crashpad crash handler. *out_installed is 0 when the handler could not start (for
 * example crashpad_handler is missing); the call itself still returns KUI_OK then. */
KUI_C_API KuiStatus kui_crash_reporter_install(const KuiCCrashReporterConfig* config, int* out_installed);
KUI_C_API KuiStatus kui_crash_reporter_uninstall(void);

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* KUI_C_H_INCLUDED */
