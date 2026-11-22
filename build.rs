use std::env;
use std::fs;
use std::path::PathBuf;

// 本 crate 自己的示例与测试在 Linux / macOS 上按「可执行文件所在目录」找 libkui_c（包里 bin/ 下与运行库放在
// 一起，与 Windows 的 DLL 搜索规则一致）。Cargo 不会把 kui-sys 的链接参数传给这里的示例和测试，所以在这里给。
// 仓库里直接 `cargo test` 时用 LD_LIBRARY_PATH / DYLD_LIBRARY_PATH 指向运行库目录（见 build.sh）。
//
// 对外仓库（git clone kui-rust）的运行库在 runtime/<平台>/lib，不在程序旁边：kui-sys 选中的运行库目录
// （links = "kui_c" 传来的 DEP_KUI_C_LIB_DIR）在本 crate 的 runtime/ 下时，再加一条指向它的搜索路径，
// `cargo run --example` / `cargo test` 不用设任何环境变量。SDK 包（native/lib）与 KUI 仓库内开发照旧只有
// $ORIGIN，打包出的预编译程序不会带构建机的绝对路径。
fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match os.as_str() {
        "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
        "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path"),
        _ => return,
    }
    let (Some(lib_dir), Some(manifest_dir)) = (
        env::var_os("DEP_KUI_C_LIB_DIR"),
        env::var_os("CARGO_MANIFEST_DIR"),
    ) else {
        return;
    };
    let (Ok(lib_dir), Ok(runtime)) = (
        fs::canonicalize(PathBuf::from(lib_dir)),
        fs::canonicalize(PathBuf::from(manifest_dir).join("runtime")),
    ) else {
        return;
    };
    if lib_dir.starts_with(&runtime) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
}
