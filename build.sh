#!/usr/bin/env bash
# Linux x64 / macOS arm64 上构建 Rust SDK 的测试与全部示例（Windows 用 build.ps1）。
#
#   ./build.sh [-o OUTPUT] [-n NATIVE_LIB_DIR] [-f FULL_PROFILE_RUNTIME_DIR]
#
# OUTPUT 默认 bin/。NATIVE_LIB_DIR 放 libkui_c 与 libkui：默认依次找解包后的 native/lib、对外仓库（git clone kui-rust）的
# runtime/<平台>/lib、仓库内开发的 ../c/build（按本版 C++ staging 构建的 sdk/c）。FULL_PROFILE_RUNTIME_DIR 放按目录加载的
# 扩展 / 媒体插件：默认依次找 native/lib、runtime/<平台>/lib、仓库内的 ../cpp/lib。只有 Release 一套。示例可执行文件按
# $ORIGIN（macOS @executable_path）找运行库，所以运行库与插件都放到输出目录里；测试用 LD_LIBRARY_PATH /
# DYLD_LIBRARY_PATH。运行测试的 Linux 机器需要显示服务器（没有桌面时用 xvfb-run）。
set -euo pipefail

SDK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT="$SDK_ROOT/bin"
NATIVE_LIB_DIR=""
FULL_PROFILE_DIR=""
while getopts "o:n:f:" option; do
    case "$option" in
        o) OUTPUT="$OPTARG" ;;
        n) NATIVE_LIB_DIR="$OPTARG" ;;
        f) FULL_PROFILE_DIR="$OPTARG" ;;
        *) echo "usage: $0 [-o OUTPUT] [-n NATIVE_LIB_DIR] [-f FULL_PROFILE_RUNTIME_DIR]" >&2; exit 2 ;;
    esac
done
case "$(uname -s)" in
    Darwin) SUFFIX=dylib; LIBRARY_PATH_VAR=DYLD_LIBRARY_PATH; PLATFORM_TAG=macos-arm64 ;;
    *) SUFFIX=so; LIBRARY_PATH_VAR=LD_LIBRARY_PATH; PLATFORM_TAG=linux-x64 ;;
esac
LIB_C="libkui_c.$SUFFIX"
LIB_CORE="libkui.$SUFFIX"
# 依次找第一个含 $1 的目录（其余参数是候选目录），找到就打印它；都没有返回 1。
first_dir_with() {
    local file="$1" dir
    shift
    for dir in "$@"; do
        if [[ -f "$dir/$file" ]]; then echo "$dir"; return 0; fi
    done
    return 1
}
if [[ -z "$NATIVE_LIB_DIR" ]]; then
    candidates=("$SDK_ROOT/native/lib" "$SDK_ROOT/runtime/$PLATFORM_TAG/lib" "$SDK_ROOT/../c/build")
    NATIVE_LIB_DIR="$(first_dir_with "$LIB_C" "${candidates[@]}")" ||
        { echo "找不到 $LIB_C（可用 -n 指定）；试过：${candidates[*]}" >&2; exit 1; }
fi
if [[ -z "$FULL_PROFILE_DIR" ]]; then
    candidates=("$SDK_ROOT/native/lib" "$SDK_ROOT/runtime/$PLATFORM_TAG/lib" "$SDK_ROOT/../cpp/lib")
    FULL_PROFILE_DIR="$(first_dir_with "libkui_ext_3d.$SUFFIX" "${candidates[@]}")" ||
        { echo "找不到 libkui_ext_3d.$SUFFIX（可用 -f 指定）；试过：${candidates[*]}" >&2; exit 1; }
fi
NATIVE_LIB_DIR="$(cd "$NATIVE_LIB_DIR" && pwd)"
FULL_PROFILE_DIR="$(cd "$FULL_PROFILE_DIR" && pwd)"
for name in "$LIB_C" "$LIB_CORE"; do
    [[ -f "$NATIVE_LIB_DIR/$name" ]] || { echo "缺少运行时：$NATIVE_LIB_DIR/$name" >&2; exit 1; }
done
mkdir -p "$OUTPUT"
OUTPUT="$(cd "$OUTPUT" && pwd)"

export KUI_C_LIB_DIR="$NATIVE_LIB_DIR"
TARGET_DIR="${CARGO_TARGET_DIR:-$SDK_ROOT/target}"
cd "$SDK_ROOT"
# canonical 示例数只读包内生成的清单（产品清单 + API 巡览示例），不在脚本里写死。
canonical_count=$(grep -m1 '"count"' examples/canonical_manifest.json | tr -dc '0-9')
[[ -n "$canonical_count" && $canonical_count -gt 0 ]] || { echo "读不到 examples/canonical_manifest.json 的 count" >&2; exit 1; }

cargo fmt --all -- --check
env "$LIBRARY_PATH_VAR=$NATIVE_LIB_DIR${!LIBRARY_PATH_VAR:+:${!LIBRARY_PATH_VAR}}" \
    cargo test --all-targets -- --test-threads=1
cargo build --release --examples

for exe in "$TARGET_DIR/release/examples"/*; do
    name="$(basename "$exe")"
    [[ -f "$exe" && -x "$exe" ]] || continue
    [[ "$name" == *.* || "$name" =~ -[0-9a-f]{16}$ ]] && continue
    cp -f "$exe" "$OUTPUT/kui_rust_$name"
done

cp -R examples/assets "$OUTPUT/"
REPO_ROOT="$(cd "$SDK_ROOT/../.." && pwd)"
if [[ -d "$REPO_ROOT/examples/html_loader/samples" ]]; then
    copy_asset() { [[ -e "$1" ]] || return 0; rm -rf "$2"; mkdir -p "$(dirname "$2")"; cp -R "$1" "$2"; }
    copy_asset "$REPO_ROOT/examples/html_loader/samples" "$OUTPUT/assets/native_samples"
    copy_asset "$REPO_ROOT/tests/script/html" "$OUTPUT/assets/script_html"
    copy_asset "$REPO_ROOT/examples/native/animation/assets/pulse.json" "$OUTPUT/assets/native_animation/pulse.json"
    copy_asset "$REPO_ROOT/resources/audio/sample.mp3" "$OUTPUT/assets/native_media/sample.mp3"
    copy_asset "$REPO_ROOT/resources/video/sample.mp4" "$OUTPUT/assets/native_media/sample.mp4"
    copy_asset "$REPO_ROOT/resources/3d models/stylized_tank.glb" "$OUTPUT/assets/native_3d/stylized_tank.glb"
    copy_asset "$REPO_ROOT/examples/font_awesome_showcase/css" "$OUTPUT/assets/pages/css"
    copy_asset "$REPO_ROOT/examples/font_awesome_showcase/webfonts" "$OUTPUT/assets/pages/webfonts"
fi

for file in "$FULL_PROFILE_DIR"/libkui_ext_*."$SUFFIX" "$FULL_PROFILE_DIR"/libkui_media_*."$SUFFIX" \
            "$FULL_PROFILE_DIR"/libpdfium."$SUFFIX" "$FULL_PROFILE_DIR"/libav* "$FULL_PROFILE_DIR"/libsw* \
            "$FULL_PROFILE_DIR"/LICENSE.FFmpeg*.txt "$FULL_PROFILE_DIR"/*.filamat; do
    [[ -f "$file" ]] && cp -f "$file" "$OUTPUT/"
done
cp -f "$NATIVE_LIB_DIR/$LIB_C" "$NATIVE_LIB_DIR/$LIB_CORE" "$OUTPUT/"

quick=$(find "$OUTPUT" -maxdepth 1 -type f -name 'kui_rust_*' ! -name 'kui_rust_canonical_*' | wc -l | tr -d ' ')
canonical=$(find "$OUTPUT" -maxdepth 1 -type f -name 'kui_rust_canonical_*' | wc -l | tr -d ' ')
sources=$(find examples -maxdepth 1 -name 'canonical_*.rs' | wc -l | tr -d ' ')
pages=$(find examples/assets/pages -maxdepth 1 -name '*.html' | wc -l | tr -d ' ')
if [[ $quick -ne 8 || $canonical -ne $canonical_count || $sources -ne $canonical_count || $pages -ne $canonical_count ]]; then
    echo "Rust bin 门失败：quick=$quick, canonical=$canonical, sources=$sources, pages=$pages" >&2
    exit 1
fi
for name in "libkui_ext_3d.$SUFFIX" "libkui_media_audio.$SUFFIX"; do
    [[ -f "$OUTPUT/$name" ]] || { echo "Rust bin 运行时门失败：缺少 $name（扩展运行时目录：$FULL_PROFILE_DIR）" >&2; exit 1; }
done
for name in "$LIB_C" "$LIB_CORE"; do
    cmp -s "$NATIVE_LIB_DIR/$name" "$OUTPUT/$name" || { echo "Rust bin 核心运行时与 $NATIVE_LIB_DIR 不一致：$name" >&2; exit 1; }
done
echo "Rust SDK build PASS: quick=$quick canonical=$canonical output=$OUTPUT"
