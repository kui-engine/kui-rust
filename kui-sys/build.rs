use std::env;
use std::path::{Path, PathBuf};

/// 目标平台对应的目录名：与发布包、对外仓库 `runtime/<平台>/` 的平台名相同。
fn platform_tag(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("macos", "aarch64") => Some("macos-arm64"),
        _ => None,
    }
}

/// 没有 `KUI_C_SDK` 时依次尝试的根目录（`manifest_dir` 是 kui-sys 目录，在包根 / 仓库根下一级）：
/// ① SDK 包布局 `<包根>/native`；② 对外仓库（`git clone kui-rust`）布局 `<仓库根>/runtime/<平台>`；
/// ③ KUI 仓库内开发 `sdk/c`（即 `<kui-sys>/../../c`，以前唯一的默认）。三种布局里导入库都在 `lib/`
/// （仓库内开发在 `build/<配置>`），见 `library_dirs()`。
fn default_roots(manifest_dir: &Path, tag: &str) -> Vec<PathBuf> {
    let sdk_root = manifest_dir.join("..");
    vec![
        sdk_root.join("native"),
        sdk_root.join("runtime").join(tag),
        manifest_dir.join("../..").join("c"),
    ]
}

/// 某个根目录下依次找导入库（Windows）/ 运行库（Linux、macOS）的子目录。
fn library_dirs(root: &Path, windows: bool, profile: &str) -> Vec<PathBuf> {
    if windows {
        let configuration = if profile == "release" {
            "Release"
        } else {
            "Debug"
        };
        // SDK 包（Rust 包的 native/、C 包根目录）与对外仓库的 runtime/windows-x64 只带 Release 导入库 lib/：
        // Rust 在 MSVC 目标上无论 profile 都链接 Release CRT，C ABI 也不跨边界传 CRT 对象，debug profile 同样
        // 链接它。仓库内开发时 sdk/c 没有 lib/，按 profile 取 sdk/c/build/<配置>。
        vec![root.join("lib"), root.join("build").join(configuration)]
    } else {
        // Linux / macOS 只有一套（没有 Debug CRT 之分）：包或仓库的 lib/，或单配置生成器构建的 sdk/c/build。
        vec![
            root.join("lib"),
            root.join("build"),
            root.join("build").join("Release"),
        ]
    }
}

/// 找导入库 / 运行库所在目录。返回 (找到的目录, 按顺序试过的全部目录)。
/// `override_dir`（`KUI_C_LIB_DIR`）最优先；给了 `explicit_root`（`KUI_C_SDK`）就只在它下面找，不再自动探测——
/// 这两个环境变量的行为与以前相同。
fn resolve_lib_dir(
    manifest_dir: &Path,
    explicit_root: Option<PathBuf>,
    override_dir: Option<PathBuf>,
    tag: &str,
    windows: bool,
    profile: &str,
    library: &str,
) -> (Option<PathBuf>, Vec<PathBuf>) {
    let roots = match explicit_root {
        Some(root) => vec![root],
        None => default_roots(manifest_dir, tag),
    };
    let mut candidates: Vec<PathBuf> = override_dir.into_iter().collect();
    for root in &roots {
        candidates.extend(library_dirs(root, windows, profile));
    }
    let found = candidates
        .iter()
        .find(|item| item.is_dir() && item.join(library).is_file())
        .cloned();
    (found, candidates)
}

fn main() {
    // 目标平台（不是运行构建脚本的这台机器）：Windows x64、Linux x64、macOS arm64。
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let library = match (os.as_str(), arch.as_str()) {
        ("windows", "x86_64") => "kui_c.lib",
        ("linux", "x86_64") => "libkui_c.so",
        ("macos", "aarch64") => "libkui_c.dylib",
        _ => panic!("kui-sys 支持 Windows x64、Linux x64 与 macOS arm64，当前目标是 {os} {arch}"),
    };
    let tag = platform_tag(&os, &arch).expect("platform tag");
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let (found, candidates) = resolve_lib_dir(
        &manifest_dir,
        env::var_os("KUI_C_SDK").map(PathBuf::from),
        env::var_os("KUI_C_LIB_DIR").map(PathBuf::from),
        tag,
        os == "windows",
        &profile,
        library,
    );
    let lib_dir = found.unwrap_or_else(|| {
        panic!(
            "找不到 {library}。自动探测依次试了 SDK 包的 native/、对外仓库的 runtime/{tag}/ 与 KUI 仓库内的 sdk/c；\
             也可以设置 KUI_C_SDK（运行库根目录，导入库在其 lib/ 下）或 KUI_C_LIB_DIR（导入库所在目录）。试过：{:?}",
            candidates
        )
    });
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=kui_c");
    // links = "kui_c"：依赖本 crate 的构建脚本可从 DEP_KUI_C_LIB_DIR 读到运行库目录。
    println!("cargo:lib_dir={}", lib_dir.display());
    println!("cargo:rerun-if-env-changed=KUI_C_SDK");
    println!("cargo:rerun-if-env-changed=KUI_C_LIB_DIR");
    println!(
        "cargo:rerun-if-changed={}",
        Path::new("include/kui_c.h").display()
    );
}

// 探测顺序的单元测试。cargo 编译构建脚本时不带 --test，这些测试不进构建；由
// tests/test_derived_sdk_packages.py 用 `rustc --test build.rs` 单独编译运行（只用标准库）。
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 在临时目录里造一个 `<根>/kui-sys` 布局，files 是相对 `<根>` 的空文件；返回 kui-sys 目录。
    fn fake_tree(files: &[&str]) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "kui-sys-build-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&root);
        let manifest = root.join("sdk").join("kui-sys");
        fs::create_dir_all(&manifest).unwrap();
        for file in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"").unwrap();
        }
        manifest
    }

    fn resolve(
        manifest: &Path,
        sdk: Option<&str>,
        lib: Option<&str>,
    ) -> (Option<PathBuf>, Vec<PathBuf>) {
        let root = manifest.join("../..");
        resolve_lib_dir(
            manifest,
            sdk.map(|value| root.join(value)),
            lib.map(|value| root.join(value)),
            "windows-x64",
            true,
            "debug",
            "kui_c.lib",
        )
    }

    fn ends_with(found: &Option<PathBuf>, tail: &str) -> bool {
        let text = found.as_ref().unwrap().to_string_lossy().replace('\\', "/");
        text.ends_with(tail)
    }

    #[test]
    fn platform_tags_match_the_package_names() {
        assert_eq!(platform_tag("windows", "x86_64"), Some("windows-x64"));
        assert_eq!(platform_tag("linux", "x86_64"), Some("linux-x64"));
        assert_eq!(platform_tag("macos", "aarch64"), Some("macos-arm64"));
        assert_eq!(platform_tag("linux", "aarch64"), None);
    }

    #[test]
    fn package_layout_is_found_first() {
        let manifest = fake_tree(&[
            "sdk/native/lib/kui_c.lib",
            "sdk/runtime/windows-x64/lib/kui_c.lib",
        ]);
        let (found, _) = resolve(&manifest, None, None);
        assert!(ends_with(&found, "/native/lib"), "{found:?}");
    }

    #[test]
    fn repository_runtime_layout_is_found_without_any_variable() {
        let manifest = fake_tree(&["sdk/runtime/windows-x64/lib/kui_c.lib"]);
        let (found, _) = resolve(&manifest, None, None);
        assert!(ends_with(&found, "/runtime/windows-x64/lib"), "{found:?}");
    }

    #[test]
    fn kui_repository_development_layout_still_works() {
        let manifest = fake_tree(&["c/build/Debug/kui_c.lib"]);
        let (found, _) = resolve(&manifest, None, None);
        assert!(ends_with(&found, "c/build/Debug"), "{found:?}");
    }

    #[test]
    fn explicit_variables_take_priority() {
        let manifest = fake_tree(&[
            "sdk/native/lib/kui_c.lib",
            "elsewhere/lib/kui_c.lib",
            "libdir/kui_c.lib",
        ]);
        let (found, _) = resolve(&manifest, Some("elsewhere"), None);
        assert!(ends_with(&found, "elsewhere/lib"), "{found:?}");
        let (found, _) = resolve(&manifest, Some("elsewhere"), Some("libdir"));
        assert!(ends_with(&found, "libdir"), "{found:?}");
        // KUI_C_SDK 指错时不悄悄退回自动探测。
        let (found, tried) = resolve(&manifest, Some("missing"), None);
        assert!(found.is_none());
        assert!(tried
            .iter()
            .all(|path| path.to_string_lossy().contains("missing")));
    }

    #[test]
    fn nothing_found_reports_every_candidate() {
        let manifest = fake_tree(&[]);
        let (found, tried) = resolve(&manifest, None, None);
        assert!(found.is_none());
        let tried: Vec<String> = tried
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(
            tried.iter().any(|path| path.ends_with("/native/lib")),
            "{tried:?}"
        );
        assert!(
            tried
                .iter()
                .any(|path| path.ends_with("runtime/windows-x64/lib")),
            "{tried:?}"
        );
        assert!(
            tried.iter().any(|path| path.ends_with("c/build/Debug")),
            "{tried:?}"
        );
    }
}
