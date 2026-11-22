#requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    # 默认 Release 输出到 bin（与包内预编译产物同一目录），Debug 输出到 bin\debug，
    # 不覆盖标准 Release 产物。
    [string]$Output = '',
    [string]$NativeSdk = '',
    # Full Profile 扩展运行时目录：kui_ext_*、kui_media_*、FFmpeg、pdfium、*.filamat。
    [string]$FullProfileRuntimeDir = ''
)

$ErrorActionPreference = 'Stop'
$SdkRoot = Split-Path -Parent $PSCommandPath
if (-not $Output) { $Output = if ($Configuration -eq 'Debug') { 'bin\debug' } else { 'bin' } }
$Output = if ([IO.Path]::IsPathRooted($Output)) { $Output } else { Join-Path $SdkRoot $Output }
if (-not $NativeSdk) {
    # 依次找：SDK 包布局 native；对外仓库（git clone kui-rust）布局 runtime\windows-x64（同样是 lib\ + bin\）；
    # KUI 仓库内开发 ..\c（导入库在 build\<配置>）。与 kui-sys/build.rs 没有 KUI_C_SDK 时的探测顺序相同。
    $nativeCandidates = @((Join-Path $SdkRoot 'native'), (Join-Path $SdkRoot 'runtime\windows-x64'))
    $repository = Join-Path $SdkRoot '..\c'
    $NativeSdk = $nativeCandidates | Where-Object { Test-Path (Join-Path $_ 'lib\kui_c.lib') } | Select-Object -First 1
    if (-not $NativeSdk) {
        if (-not (Test-Path (Join-Path $repository 'build'))) {
            throw "找不到 C ABI 导入库 lib\kui_c.lib（可用 -NativeSdk 指定）；试过：$(($nativeCandidates + (Join-Path $repository 'build')) -join '；')"
        }
        $NativeSdk = $repository
    }
}
$NativeSdk = [IO.Path]::GetFullPath($NativeSdk)
$cargo = (Get-Command cargo -ErrorAction Stop).Source
$env:KUI_C_SDK = $NativeSdk
$oldPath = $env:PATH
# 运行时必须与 kui-sys/build.rs 选的导入库同出一处：build.rs 先找 <NativeSdk>\lib，再找 <NativeSdk>\build\<cfg>。
# - 包布局（有 lib\kui_c.lib：Rust 包的 native\，或 C 包根目录）：公开包只带 Release 导入库与运行库（lib、bin），
#   debug profile 也链接、运行它们——Rust 在 MSVC 目标上本来就总用 Release CRT，C ABI 也不跨边界传 CRT 对象。
# - 仓库布局（..\c）：导入库在 build\<cfg>，运行时也必须取 build\<cfg>。仓库里的 sdk\c\bin 是上一次
#   发版同步回来的旧 DLL，导出可能比新编的导入库少；以前优先取它，C ABI 新增导出后测试就起不来。
$packagedLayout = Test-Path (Join-Path $NativeSdk 'lib\kui_c.lib')
if ($packagedLayout) {
    $releaseRuntime = Join-Path $NativeSdk 'bin'
    $debugRuntime = $releaseRuntime
} else {
    $releaseRuntime = Join-Path $NativeSdk 'build\Release'
    $debugRuntime = Join-Path $NativeSdk 'build\Debug'
}
if (-not (Test-Path (Join-Path $debugRuntime 'kui_c.dll'))) { throw "缺少 Rust 测试（debug profile）所需 C ABI 运行时：$debugRuntime" }
if (-not (Test-Path (Join-Path $releaseRuntime 'kui_c.dll'))) { throw "缺少 Rust example 所需 Release C ABI 运行时：$releaseRuntime" }
# 核心运行时（kui_c.dll + kui.dll）与 build.rs 为该 profile 选的导入库同出一处：仓库布局里 Debug 构建链接
# build\Debug 的导入库、运行时也取 build\Debug（以前 Debug 输出里放的是 Release 核心）；包布局只有 Release 一套。
$coreRuntimeDir = if ($Configuration -eq 'Debug') { $debugRuntime } else { $releaseRuntime }
$coreRuntime = @('kui_c.dll', 'kui.dll')
foreach ($name in $coreRuntime) {
    if (-not (Test-Path (Join-Path $coreRuntimeDir $name))) { throw "缺少运行时：$coreRuntimeDir\$name" }
}
# 扩展与媒体插件只发 Release 版。它们和核心之间走纯 C 协议（kuiext_* / kui_media_*），
# 不跨 CRT 传 C++ 对象，所以 Debug 核心（/MDd）加载 Release 扩展是受支持的组合——
# 与 C++ SDK（bin/debug 只有 kui.dll）和 C SDK 的 kui_c_copy_runtime_dlls 相同。
# 包布局取 <NativeSdk>\bin；仓库布局取本仓 C++ SDK staging 的 bin（不取同样是旧同步物的 sdk\c\bin）。
if (-not $FullProfileRuntimeDir) {
    $FullProfileRuntimeDir = if ($packagedLayout) { $releaseRuntime } else { Join-Path $SdkRoot '..\cpp\bin' }
}
$FullProfileRuntimeDir = [IO.Path]::GetFullPath($FullProfileRuntimeDir)

Push-Location $SdkRoot
try {
    & $cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check 失败：$LASTEXITCODE" }
    $env:PATH = "$debugRuntime;$oldPath"
    & $cargo test --all-targets -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw "cargo test 失败：$LASTEXITCODE" }
    if ($Configuration -eq 'Release') {
        & $cargo build --release --examples
    } else {
        & $cargo build --examples
    }
    if ($LASTEXITCODE -ne 0) { throw "cargo build --examples 失败：$LASTEXITCODE" }
    New-Item -ItemType Directory -Path $Output -Force | Out-Null
    $profileDir = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
    # 与 build.sh 一致：cargo 认 CARGO_TARGET_DIR，产物就在那里（包级验收把它设到解包目录外）。
    $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $SdkRoot 'target' }
    Get-ChildItem (Join-Path $targetDir "$profileDir\examples") -Filter '*.exe' -File |
        Where-Object { $_.Name -notmatch '-[0-9a-f]{16}\.exe$' } |
        ForEach-Object { Copy-Item -Force $_.FullName (Join-Path $Output ("kui_rust_{0}" -f $_.Name)) }
    Copy-Item -Recurse -Force (Join-Path $SdkRoot 'examples\assets') (Join-Path $Output 'assets')
    $repositoryRoot = [IO.Path]::GetFullPath((Join-Path $SdkRoot '..\..'))
    if (Test-Path -LiteralPath (Join-Path $repositoryRoot 'examples\html_loader\samples')) {
        $assetCopies = @(
            @((Join-Path $repositoryRoot 'examples\html_loader\samples'), (Join-Path $Output 'assets\native_samples')),
            @((Join-Path $repositoryRoot 'tests\script\html'), (Join-Path $Output 'assets\script_html')),
            @((Join-Path $repositoryRoot 'examples\native\animation\assets\pulse.json'), (Join-Path $Output 'assets\native_animation\pulse.json')),
            @((Join-Path $repositoryRoot 'resources\audio\sample.mp3'), (Join-Path $Output 'assets\native_media\sample.mp3')),
            @((Join-Path $repositoryRoot 'resources\video\sample.mp4'), (Join-Path $Output 'assets\native_media\sample.mp4')),
            @((Join-Path $repositoryRoot 'resources\3d models\stylized_tank.glb'), (Join-Path $Output 'assets\native_3d\stylized_tank.glb')),
            @((Join-Path $repositoryRoot 'examples\font_awesome_showcase\css'), (Join-Path $Output 'assets\pages\css')),
            @((Join-Path $repositoryRoot 'examples\font_awesome_showcase\webfonts'), (Join-Path $Output 'assets\pages\webfonts'))
        )
        foreach ($copy in $assetCopies) {
            if (Test-Path -LiteralPath $copy[0]) {
                New-Item -ItemType Directory -Path (Split-Path -Parent $copy[1]) -Force | Out-Null
                Copy-Item -Recurse -Force -LiteralPath $copy[0] -Destination $copy[1]
            }
        }
    }
    # 先铺 Release 扩展运行时（跳过核心两个 DLL），再放与本配置匹配的核心。
    $extensionFiles = @(Get-ChildItem -LiteralPath $FullProfileRuntimeDir -File | Where-Object {
        # FFmpeg 的许可文本与 FFmpeg DLL 同行（T-DEBT-SDK-WINDOWS-FFMPEG-GPL-BUILD）。
        ($_.Extension -eq '.dll' -or $_.Extension -eq '.filamat' -or $_.Name -like 'LICENSE.FFmpeg*.txt') -and
            $coreRuntime -notcontains $_.Name
    })
    foreach ($runtimeFile in $extensionFiles) {
        Copy-Item -Force -LiteralPath $runtimeFile.FullName -Destination $Output
    }
    foreach ($name in $coreRuntime) {
        Copy-Item -Force -LiteralPath (Join-Path $coreRuntimeDir $name) -Destination $Output
    }
    $quickBins = @(Get-ChildItem $Output -Filter 'kui_rust_*.exe' -File | Where-Object {
        $_.Name -notlike 'kui_rust_canonical_*'
    })
    $canonicalBins = @(Get-ChildItem $Output -Filter 'kui_rust_canonical_*.exe' -File)
    # canonical 示例数只读包内生成的清单（产品清单 + API 巡览示例），不在脚本里写死。
    $canonicalManifest = [IO.File]::ReadAllText((Join-Path $SdkRoot 'examples\canonical_manifest.json'), [Text.Encoding]::UTF8) | ConvertFrom-Json
    $canonicalCount = [int]$canonicalManifest.count
    if ($canonicalCount -le 0 -or $canonicalCount -ne @($canonicalManifest.examples).Count) {
        throw "examples\canonical_manifest.json 的 count=$canonicalCount 与 examples 列表不一致"
    }
    $canonicalSources = @(Get-ChildItem (Join-Path $SdkRoot 'examples') -Filter 'canonical_*.rs' -File)
    $canonicalPages = @(Get-ChildItem (Join-Path $SdkRoot 'examples\assets\pages') -Filter '*.html' -File)
    if ($quickBins.Count -ne 8 -or $canonicalBins.Count -ne $canonicalCount -or
        $canonicalSources.Count -ne $canonicalCount -or $canonicalPages.Count -ne $canonicalCount) {
        throw "Rust bin 门失败：quick=$($quickBins.Count), canonical=$($canonicalBins.Count), sources=$($canonicalSources.Count), pages=$($canonicalPages.Count)"
    }
    foreach ($runtimeName in @('kui_ext_3d.dll', 'kui_media_audio.dll')) {
        if (-not (Test-Path -LiteralPath (Join-Path $Output $runtimeName))) {
            throw "Rust bin 运行时门失败：缺少 $runtimeName（扩展运行时目录：$FullProfileRuntimeDir）"
        }
    }
    foreach ($name in $coreRuntime) {
        $expected = (Get-FileHash -LiteralPath (Join-Path $coreRuntimeDir $name) -Algorithm SHA256).Hash
        $actual = (Get-FileHash -LiteralPath (Join-Path $Output $name) -Algorithm SHA256).Hash
        if ($expected -ne $actual) {
            throw "Rust bin 运行时门失败：$name 不是 $Configuration 核心运行时（$coreRuntimeDir）"
        }
    }
    Write-Host "RUST SDK BUILD PASS: configuration=$Configuration, $(& $cargo --version), quick=8, canonical=$canonicalCount, core=$coreRuntimeDir, extensions=$FullProfileRuntimeDir, output=$Output" -ForegroundColor Green
} finally {
    $env:PATH = $oldPath
    Pop-Location
}
