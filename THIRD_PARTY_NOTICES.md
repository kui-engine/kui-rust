# Third-Party Notices

The KUI SDK packages incorporate components from the following third-party projects. Each component is subject to its own license terms, which are preserved in this document.

KUI itself is distributed under the [MIT License](./LICENSE). Bundling KUI redistributes the third-party components below; you must comply with their respective license terms when distributing your KUI-based application.

This one file ships unchanged in every SDK package. The packages share the same native runtime, so the native sections apply to all of them; the language-runtime section applies only to the package named there.

| Package | Native runtime it carries | Additional components |
|---|---|---|
| `kui-cpp-sdk` | `bin/`: `kui.dll`, extension and media DLLs, FFmpeg DLLs with their license texts, `pdfium.dll` | — |
| `kui-c-sdk` (C ABI) | `bin/`: the same native runtime plus `kui_c.dll` | miniaudio source under `extensions/kui_media_audio/third_party/` |
| `kui-cpp-wrapper-sdk` (source C++ Wrapper over the C ABI) | `bin/`: the same native runtime plus `kui_c.dll` (byte-identical to `kui-c-sdk`) | — |
| `kui-go-sdk` | `native/bin/` (Linux / macOS `native/lib/`): the same native runtime plus `kui_c.dll` | — |
| `kui-rust-sdk` | `native/bin/` (Linux / macOS `native/lib/`): the same native runtime plus `kui_c.dll` | — |
| `kui-c-examples`, `kui-cpp-wrapper-examples` (prebuilt examples) | `bin/` (Linux / macOS also `lib/`): byte-identical copy of the matching SDK package's runtime | — |
| `kui-go-examples` | `bin/`: byte-identical copy of `kui-go-sdk`'s `native/` runtime | Go runtime and standard library in the prebuilt `bin/kui_go_*` programs |
| `kui-rust-examples` | `bin/`: byte-identical copy of `kui-rust-sdk`'s `native/` runtime | Rust standard library in the prebuilt `bin/kui_rust_*` programs |

---

## Statically Linked into `kui.dll`

These components are compiled into the main `kui.dll` and contribute to its statically linked footprint.

### Skia — BSD-3-Clause

**Project:** https://skia.org/
**License:** BSD-3-Clause
**Usage:** 2D rendering, text shaping, vector graphics, image decoding, Lottie/Bodymovin animation playback (via the bundled Skottie module, as of R-28/T-LOTTIE-SKOTTIE-001 — replaces the formerly separate rlottie dependency)

```
Copyright (c) 2011 Google Inc. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

    * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
    * Neither the name of Google Inc. nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### HarfBuzz — MIT

**Project:** https://harfbuzz.github.io/
**License:** MIT (with additional Old MIT-style clause)
**Usage:** Text shaping for complex scripts

Excerpt:

```
HarfBuzz is licensed under the so-called "Old MIT" license:

  Permission is hereby granted, without written agreement and without
  license or royalty fees, to use, copy, modify, and distribute this
  software and its documentation for any purpose, provided that the
  above copyright notice and the following two paragraphs appear in
  all copies of this software.

  IN NO EVENT SHALL THE COPYRIGHT HOLDER BE LIABLE TO ANY PARTY FOR
  DIRECT, INDIRECT, SPECIAL, INCIDENTAL, OR CONSEQUENTIAL DAMAGES
  ARISING OUT OF THE USE OF THIS SOFTWARE AND ITS DOCUMENTATION, EVEN
  IF THE COPYRIGHT HOLDER HAS BEEN ADVISED OF THE POSSIBILITY OF SUCH
  DAMAGE.
```

### Lua — MIT

**Project:** https://www.lua.org/
**License:** MIT
**Usage:** Embedded scripting engine (the Windows packages carry Lua 5.5; scripts can read the exact version from `_VERSION`)

```
Copyright (C) 1994-2024 Lua.org, PUC-Rio.

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

### sol2 — MIT

**Project:** https://github.com/ThePhD/sol2
**License:** MIT
**Usage:** Lua / C++ bindings

(Same MIT text as above; author: ThePhD.)

### ICU — Unicode-DFS-2016

**Project:** http://site.icu-project.org/
**License:** Unicode Data Files and Software License
**Usage:** Unicode services for text rendering

Full license at https://www.unicode.org/license.txt.

### LunaSVG — MIT

**Project:** https://github.com/sammycage/lunasvg
**License:** MIT
**Usage:** SVG rendering

### Libraries pulled in by the static vcpkg build

`kui.dll` is linked against the static (`x64-windows-static-md`) vcpkg builds of the components above, so their own dependencies end up inside `kui.dll` as well. A static linker only keeps the objects that are referenced, so a few of the libraries below may contribute little or no code; they are listed so that this file covers the whole dependency set.

| Component | License | Copyright | Pulled in by |
|---|---|---|---|
| zlib | Zlib | (C) 1995-2024 Jean-loup Gailly and Mark Adler | Skia, KUI WOFF decoding, libpng |
| libpng | PNG Reference Library License v2 (libpng-2.0) | (c) 1995-2026 The PNG Reference Library Authors and contributors | Skia |
| libjpeg-turbo | IJG License + BSD-3-Clause (SIMD parts: Zlib) | (C) 2009-2025 D. R. Commander; (C) 2015 Viktor Szathmáry; this software is based in part on the work of the Independent JPEG Group | Skia |
| libwebp | BSD-3-Clause | (c) 2010, Google Inc. | Skia |
| Wuffs | Apache-2.0 | The Wuffs Authors | Skia (bundled) |
| DNG SDK | Adobe DNG SDK License | Adobe Systems Incorporated — "This product includes DNG technology under license by Adobe Systems Incorporated." | Skia (bundled) |
| PIEX | Apache-2.0 | Google Inc. | Skia (bundled) |
| Expat | MIT | (c) 1998-2000 Thai Open Source Software Center Ltd and Clark Cooper; (c) 2001-2025 Expat maintainers | Skia |
| Abseil | Apache-2.0 | The Abseil Authors | Skia |
| FreeType | FreeType License (FTL), chosen from FTL / GPLv2 | Portions of this software are copyright © The FreeType Project (www.freetype.org). All rights reserved. | HarfBuzz |
| bzip2 | bzip2-1.0.6 (BSD-style) | (C) 1996-2019 Julian R Seward | FreeType |
| Brotli | MIT | (c) 2009, 2010, 2013-2016 by the Brotli Authors | WOFF2, FreeType |
| WOFF2 | MIT | (c) 2013-2017 by the WOFF2 Authors | KUI WOFF2 webfont decoding |
| PlutoVG | MIT | (c) 2020-2025 Samuel Ugochukwu | LunaSVG |

Full license texts: Zlib https://zlib.net/zlib_license.html · libpng http://www.libpng.org/pub/png/src/libpng-LICENSE.txt · libjpeg-turbo https://github.com/libjpeg-turbo/libjpeg-turbo/blob/main/LICENSE.md · Apache-2.0 http://www.apache.org/licenses/LICENSE-2.0 · DNG SDK https://helpx.adobe.com/camera-raw/digital-negative.html · FTL https://gitlab.freedesktop.org/freetype/freetype/-/blob/master/docs/FTL.TXT · bzip2 https://sourceware.org/bzip2/ · MIT and BSD-3-Clause texts as reproduced above, with the copyright lines from this table.

### googletest — BSD-3-Clause (test only)

**Project:** https://github.com/google/googletest
**License:** BSD-3-Clause
**Usage:** Internal unit tests (not redistributed in SDK binaries)

---

## Optional Plugin DLLs

These components live in separate DLLs that are loaded only when needed.

### Filament — Apache-2.0 (`kui_ext_3d.dll` + `procedural_pbr.filamat`)

**Project:** https://github.com/google/filament
**License:** Apache-2.0
**Usage:** 3D rendering engine

Full Apache 2.0 text: http://www.apache.org/licenses/LICENSE-2.0

Excerpt:

```
Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

### Bullet3 — zlib (statically linked into `kui_ext_3d.dll`)

**Project:** https://github.com/bulletphysics/bullet3
**License:** zlib
**Usage:** Rigid-body physics simulation

```
Bullet Continuous Collision Detection and Physics Library
Copyright (c) 2003-2018 Erwin Coumans  http://bulletphysics.org

This software is provided 'as-is', without any express or implied warranty.
In no event will the authors be held liable for any damages arising from
the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

   1. The origin of this software must not be misrepresented; you must
      not claim that you wrote the original software. If you use this
      software in a product, an acknowledgment in the product documentation
      would be appreciated but is not required.

   2. Altered source versions must be plainly marked as such, and must
      not be misrepresented as being the original software.

   3. This notice may not be removed or altered from any source distribution.
```

### PDFium — BSD-3-Clause (`kui_ext_pdf.dll` + `pdfium.dll`)

**Project:** https://pdfium.googlesource.com/pdfium/
**License:** BSD-3-Clause (same as Skia, different copyright)

```
Copyright 2014 The PDFium Authors. All rights reserved.
[BSD-3-Clause text follows, same form as Skia]
```

### tree-sitter — MIT (`kui_ext_editor.dll`)

**Project:** https://tree-sitter.github.io/tree-sitter/
**License:** MIT
**Usage:** Incremental parsing for the code editor

```
Copyright (c) 2018 Max Brunsfeld

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:
...
```

### PCRE2 — BSD-3-Clause (statically linked into `kui_ext_editor.dll`)

**Project:** https://www.pcre.org/
**License:** BSD-3-Clause
**Usage:** Regex engine for the code editor's find/replace

### miniaudio — MIT-0 / Public Domain (`kui_media_audio.dll`)

**Project:** https://miniaud.io/
**License:** Dual: MIT-0 OR Public Domain (choose)
**Usage:** Audio playback

```
miniaudio is dual-licensed under the public domain (Unlicense) and MIT-0.
You can choose whichever license suits your needs.
```

### FFmpeg — LGPL-3.0-or-later (shared libraries beside `kui_media_video`)

**Project:** https://ffmpeg.org/
**Version:** Windows and Linux: FFmpeg `n9.0.2-39-g7adfbe40dc` (release/9.0 branch), the LGPL shared build of [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) release `autobuild-2026-10-31-12-59` (both platforms carry builds of the same FFmpeg commit). macOS: FFmpeg `9.0.2` built from the official, PGP-signed release tarball (https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz) with the same LGPL version 3 configuration (`--enable-version3`, no `--enable-gpl` / `--enable-nonfree`, no external libraries beyond the macOS system frameworks, zlib and bzip2); the exact configure options are in `extensions/deps/deps.json`. All three are the 9.0 release line with the same ABI major versions.
**License:** LGPL-3.0-or-later. This is an **LGPL build**: configured without `--enable-gpl` and without `--enable-nonfree`, so no GPL-only or non-free component is compiled in; the license string FFmpeg embeds in every library reads `LGPL version 3 or later`. The upstream build is configured with `--enable-version3`, which raises FFmpeg's license from LGPL version 2.1 to LGPL version 3 (it allows linking libraries that are only compatible with version 3). Version 3 of the LGPL is the GNU GPL version 3 plus the LGPL's additional permissions, so both license texts apply.
**Usage:** Video decoding (including the audio track of video files) for `<video>`, in `kui_media_video` only. `kui_media_audio` does not use FFmpeg (it compiles in miniaudio).
**License texts shipped:** `LICENSE.FFmpeg.txt` (GNU LGPL version 3) and `LICENSE.FFmpeg.GPLv3.txt` (GNU GPL version 3), in every directory that holds the FFmpeg libraries.
**Corresponding source:** Windows / Linux: https://github.com/FFmpeg/FFmpeg/tree/7adfbe40dcde598f93a6963d2f7b22cd75202ccc (build scripts: https://github.com/BtbN/FFmpeg-Builds). macOS: https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz (configure options: the `Darwin` entry of `extensions/deps/deps.json`).

**Files.** Five separate shared libraries beside the video plugin; the file names carry FFmpeg's ABI major version (currently avcodec 63, avformat 63, avutil 61, swresample 7, swscale 10):

- Windows: `avcodec-63.dll`, `avformat-63.dll`, `avutil-61.dll`, `swresample-7.dll`, `swscale-10.dll` — `bin/` in `kui-cpp-sdk`, `kui-c-sdk`, `kui-cpp-wrapper-sdk` and all four examples packages, `native/bin/` in `kui-go-sdk` / `kui-rust-sdk`.
- Linux: the same libraries under their SONAMEs (`libavcodec.so.63`, `libavformat.so.63`, `libavutil.so.61`, `libswresample.so.7`, `libswscale.so.10`) — `lib/` in `kui-cpp-sdk`, `bin/` in `kui-c-sdk`, `kui-cpp-wrapper-sdk` and all four examples packages, `native/lib/` in `kui-go-sdk` / `kui-rust-sdk`.
- macOS: `libavcodec.63.dylib`, `libavformat.63.dylib`, `libavutil.61.dylib`, `libswresample.7.dylib`, `libswscale.10.dylib` (install names `@rpath/…`) — `lib/` in `kui-cpp-sdk`, `bin/` in `kui-c-sdk`, `kui-cpp-wrapper-sdk` and all four examples packages, `native/lib/` in `kui-go-sdk` / `kui-rust-sdk`.

The plugin loads them from its own directory (Windows: the host puts the plugin's directory on the DLL search path while loading it; Linux: the plugin's `$ORIGIN` RUNPATH; macOS: the plugin's `@loader_path` LC_RPATH). The SDK release process rejects any FFmpeg library whose embedded license is not LGPL or whose configure line contains `--enable-gpl` / `--enable-nonfree` (`scripts/check_ffmpeg_license.py`).

**LGPL compliance note.** FFmpeg is dynamically linked and stays a set of separately replaceable shared libraries, which is what the LGPL requires for using it in a closed-source application. If you distribute your application with video support:

1. **Ship the FFmpeg libraries unmodified**, or rebuild them yourself from the corresponding source under the LGPL.
2. **Ship `LICENSE.FFmpeg.txt` and `LICENSE.FFmpeg.GPLv3.txt` with them** and state that your application uses FFmpeg under the LGPL. `kui_copy_runtime_dlls()` (C++, component `MediaVideo`) and the C SDK's runtime deployment copy both texts next to the libraries.
3. **Keep them replaceable** — end users must be able to substitute their own LGPL builds of the same ABI major version (same file names).
4. **Make the corresponding source available** (the FFmpeg commit above) as the LGPL requires for distributed object code.
5. LGPL version 3 also carries the GPLv3 "Installation Information" requirement when the combined work is shipped inside a consumer device; check whether that applies to your product.

If you do not need video, **do not deploy `kui_media_video` and do not ship the FFmpeg libraries** (C++: do not use the `MediaVideo` component; C, Go and Rust: leave out `kui_media_video` and the FFmpeg libraries when you copy the runtime); you then have no LGPL obligations. Audio-only playback (`kui_media_audio`) needs no FFmpeg.

---

## Language Runtimes in the Go and Rust Packages

These apply only to the prebuilt example programs in `kui-go-examples` / `kui-rust-examples` and to applications you build with `kui-go-sdk` / `kui-rust-sdk`. The native DLLs above are unchanged.

### Go runtime and standard library — BSD-3-Clause (`kui-go-examples`, `bin/kui_go_*`)

**Project:** https://go.dev/
**License:** BSD-3-Clause
**Usage:** Every Go executable statically links the Go runtime and the parts of the standard library it uses.

```
Copyright 2009 The Go Authors.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * Neither the name of Google LLC nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### Rust standard library — MIT OR Apache-2.0 (`kui-rust-examples`, `bin/kui_rust_*`)

**Project:** https://www.rust-lang.org/
**License:** MIT OR Apache-2.0 (choose either)
**Usage:** Every Rust executable statically links the parts of `std` (and its bundled dependencies) it uses. The `kui` and `kui-sys` crates themselves have no third-party crate dependencies (see `Cargo.lock`).

```
Copyright (c) The Rust Project Contributors

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

Apache-2.0 alternative: http://www.apache.org/licenses/LICENSE-2.0

The full list of components inside the Rust standard library is in `share/doc/rust/COPYRIGHT-library.html` of the Rust toolchain that built the programs (rustup: `~/.rustup/toolchains/<toolchain>/share/doc/rust/`).

---

## License Compatibility Summary

| Component | License | Permissive? | Commercial use OK? | Static link OK? | Notes |
|-----------|---------|-------------|---------------------|-----------------|-------|
| KUI core | MIT | ✅ | ✅ | ✅ | — |
| Skia | BSD-3-Clause | ✅ | ✅ | ✅ | — |
| HarfBuzz | MIT-style | ✅ | ✅ | ✅ | — |
| Lua | MIT | ✅ | ✅ | ✅ | — |
| sol2 | MIT | ✅ | ✅ | ✅ | — |
| ICU | Unicode-DFS | ✅ | ✅ | ✅ | — |
| LunaSVG | MIT | ✅ | ✅ | ✅ | — |
| Filament | Apache-2.0 | ✅ | ✅ | ✅ | — |
| Bullet3 | zlib | ✅ | ✅ | ✅ | — |
| PDFium | BSD-3-Clause | ✅ | ✅ | ✅ | — |
| tree-sitter | MIT | ✅ | ✅ | ✅ | — |
| PCRE2 | BSD-3-Clause | ✅ | ✅ | ✅ | — |
| miniaudio | MIT-0 / PD | ✅ | ✅ | ✅ | — |
| FFmpeg | LGPL-3.0+ | ⚠️ | ✅ (dynamic) | ❌ | Video plugin only; dynamic only, ship both license texts; see the LGPL compliance note above. |
| zlib, bzip2 | Zlib / bzip2-1.0.6 | ✅ | ✅ | ✅ | Via the static vcpkg build. |
| libpng, libjpeg-turbo, libwebp | libpng-2.0 / IJG + BSD-3-Clause / BSD-3-Clause | ✅ | ✅ | ✅ | Skia image codecs. |
| Wuffs, PIEX, Abseil | Apache-2.0 | ✅ | ✅ | ✅ | Skia. |
| DNG SDK | Adobe DNG SDK License | ✅ | ✅ | ✅ | Skia; keep the Adobe attribution line. |
| Expat, Brotli, WOFF2, PlutoVG | MIT | ✅ | ✅ | ✅ | Via the static vcpkg build. |
| FreeType | FTL | ✅ | ✅ | ✅ | FTL requires crediting the FreeType Project in your documentation. |
| Go runtime | BSD-3-Clause | ✅ | ✅ | ✅ | Go package only. |
| Rust std | MIT OR Apache-2.0 | ✅ | ✅ | ✅ | Rust package only. |
| googletest | BSD-3-Clause | ✅ | (test only) | (test only) | Not redistributed in SDK binaries. |

**Bottom line for commercial use:**
- Lottie/Bodymovin animation playback (`<lottie-player>`) is implemented via Skia's bundled Skottie module (BSD-3-Clause) as of R-28/T-LOTTIE-SKOTTIE-001 — it carries no separate license obligations beyond Skia's own.
- If you ship **without** the video plugin (C++: `Kui::MediaVideo`; C, Go, Rust: `kui_media_video` and the FFmpeg libraries):
  → Only permissive licenses involved (MIT / BSD / Apache / zlib / Unicode / MIT-0). Commercial use, including closed source, is straightforward. The audio plugin uses miniaudio only.
- If you ship **with** video:
  → You must satisfy the LGPL (version 3) for FFmpeg: keep it a separately replaceable shared library (already the case), ship its two license texts and make its source available — see the LGPL compliance note above.

When in doubt, consult a lawyer.

---

# Additional Third-Party Notices

The sections above describe what the KUI packages carry. The sections below complete them for content that ships in the SDK packages and in these repositories but is not described above. Everything here applies to every package and repository that contains the content in question. The license texts are in the [`licenses/`](./licenses/) directory next to this file.

## Example assets

### Font Awesome 6 Free fonts — SIL OFL 1.1 (fonts), CC BY 4.0 (icons), MIT (code)

**Files:** `fa-solid-900.ttf`, `fa-regular-400.ttf` and `fa-brands-400.ttf` (example assets: `assets/pages/webfonts/`, `assets/native_samples/fonts/`).
**Project:** https://fontawesome.com/ — Font Awesome Free by Fonticons, Inc. The fonts carry the copyright string "Copyright (c) Font Awesome".
**Licenses:** the font files are licensed under the SIL Open Font License 1.1 ([`licenses/font-awesome/OFL-1.1.txt`](./licenses/font-awesome/OFL-1.1.txt)); the icons are licensed under Creative Commons Attribution 4.0 (https://creativecommons.org/licenses/by/4.0/); the code is MIT. See https://fontawesome.com/license/free.
**Trademarks:** `fa-brands-400.ttf` contains the logos of third parties. Those logos are trademarks of their respective owners; including them in an example does not imply any endorsement, and the fonts' licenses do not grant any trademark rights.

### 3D model "Stylized Tank" — CC BY 4.0

**File:** `stylized_tank.glb` (example assets: `assets/native_3d/`).
**Author:** makcutka250 (https://sketchfab.com/makcutka250). **Source:** https://sketchfab.com/3d-models/stylized-tank-bb4be3e59cf84ee4b842da57d97cf1ce — the file was downloaded from Sketchfab (its metadata names Sketchfab as the glTF generator) and carries this attribution in its `asset.extras`.
**License:** Creative Commons Attribution 4.0 International — https://creativecommons.org/licenses/by/4.0/

### Sample audio and video

`sample.mp4` and `sample.mp3` (example assets: `assets/native_media/`) were generated with AI tools for the KUI project. They contain no third-party material and are distributed with the examples under this repository's MIT license.

## Components statically linked into the KUI plugin libraries

### tree-sitter grammars — MIT (`kui_ext_editor`)

Besides the tree-sitter core listed above, `kui_ext_editor` has six grammars compiled in: tree-sitter-c 0.21.4, tree-sitter-cpp 0.22.3, tree-sitter-javascript 0.21.4, tree-sitter-python 0.21.0 and tree-sitter-typescript 0.21.2 (Copyright Max Brunsfeld), and tree-sitter-lua (Copyright 2021 Munif Tanjim). License texts:
[c](./licenses/tree-sitter-grammars/tree-sitter-c.LICENSE) ·
[cpp](./licenses/tree-sitter-grammars/tree-sitter-cpp.LICENSE) ·
[javascript](./licenses/tree-sitter-grammars/tree-sitter-javascript.LICENSE) ·
[lua](./licenses/tree-sitter-grammars/tree-sitter-lua.LICENSE) ·
[python](./licenses/tree-sitter-grammars/tree-sitter-python.LICENSE) ·
[typescript](./licenses/tree-sitter-grammars/tree-sitter-typescript.LICENSE).

### PDFium and the libraries built into it — (`pdfium.dll`, `libpdfium.so`, `libpdfium.dylib`)

`kui_ext_pdf` loads PDFium, which KUI takes from the pre-built binaries of https://github.com/bblanchon/pdfium-binaries (MIT, Copyright 2014-2025 Benoit Blanchon — [`licenses/pdfium/pdfium-binaries.LICENSE`](./licenses/pdfium/pdfium-binaries.LICENSE)). PDFium itself is BSD-3-Clause ([`pdfium.txt`](./licenses/pdfium/pdfium.txt)). The pre-built library statically contains the following libraries; each license text is in [`licenses/pdfium/`](./licenses/pdfium/):

| Library | License text |
|---|---|
| Abseil | [`abseil.txt`](./licenses/pdfium/abseil.txt) (Apache-2.0) |
| AGG 2.3 | [`agg23.txt`](./licenses/pdfium/agg23.txt) |
| fast_float | [`fast_float.txt`](./licenses/pdfium/fast_float.txt) (MIT) |
| FreeType | [`freetype.txt`](./licenses/pdfium/freetype.txt) (FreeType License) |
| ICU | [`icu.txt`](./licenses/pdfium/icu.txt) (Unicode License v3) |
| Little CMS | [`lcms.txt`](./licenses/pdfium/lcms.txt) (MIT) |
| libjpeg-turbo | [`libjpeg_turbo.md`](./licenses/pdfium/libjpeg_turbo.md), [`libjpeg_turbo.ijg`](./licenses/pdfium/libjpeg_turbo.ijg) (IJG, BSD-3-Clause, Zlib) |
| OpenJPEG | [`libopenjpeg.txt`](./licenses/pdfium/libopenjpeg.txt) (BSD) |
| libpng | [`libpng.txt`](./licenses/pdfium/libpng.txt) (libpng License) |
| libtiff | [`libtiff.txt`](./licenses/pdfium/libtiff.txt) |
| llvm-libc | [`llvm-libc.txt`](./licenses/pdfium/llvm-libc.txt) (Apache-2.0 with LLVM Exceptions) |
| simdutf | [`simdutf.txt`](./licenses/pdfium/simdutf.txt) (MIT) |
| zlib | [`zlib.txt`](./licenses/pdfium/zlib.txt) (Zlib) |

### Filament — Apache-2.0 (`kui_ext_3d`)

The complete Apache License 2.0 is in [`licenses/filament/LICENSE`](./licenses/filament/LICENSE). `kui_ext_3d` links these Filament static libraries (all Filament code under Apache-2.0): filament, backend, bluegl, bluevk, filabridge, filaflat, utils, geometry, ibl, camutils, shaders, viewer, gltfio, gltfio_core, uberzlib, uberarchive and ktxreader. Together with them it links the following third-party components that Filament ships:

| Component | License text |
|---|---|
| smol-v | [`smol-v.LICENSE`](./licenses/filament/third_party/smol-v.LICENSE) (MIT or public domain) |
| meshoptimizer | [`meshoptimizer.LICENSE`](./licenses/filament/third_party/meshoptimizer.LICENSE) (MIT) |
| stb | [`stb.LICENSE`](./licenses/filament/third_party/stb.LICENSE) (public domain / MIT) |
| zstd | [`zstd.LICENSE`](./licenses/filament/third_party/zstd.LICENSE) (BSD-3-Clause; zstd is dual-licensed BSD / GPLv2 and KUI uses it under the BSD license) |
| Draco | [`draco.LICENSE`](./licenses/filament/third_party/draco.LICENSE) (Apache-2.0) |
| Basis Universal | [`basisu.LICENSE`](./licenses/filament/third_party/basisu.LICENSE) (Apache-2.0) |
| MikkTSpace | [`mikktspace.LICENSE`](./licenses/filament/third_party/mikktspace.LICENSE) (Zlib-style, from the source header) |

## FFmpeg: libraries built into the Windows and Linux builds

The Windows and Linux FFmpeg libraries are the BtbN/FFmpeg-Builds LGPL shared build named above (https://github.com/BtbN/FFmpeg-Builds/releases/tag/autobuild-2026-10-31-12-59; its build scripts name the upstream source and version of each library). That build is configured with `--enable-version3` and without `--enable-gpl` and `--enable-nonfree`, and FFmpeg's `configure` only accepts external libraries whose licenses are compatible with the license it is configured for (here the LGPL version 3 or later). The external libraries enabled in that configuration are:

- **Windows and Linux:** amf, chromaprint, ffnvcodec, fontconfig, gmp, iconv, lcms2, libaom, libaribb24, libaribcaption, libass, libbluray, libdav1d, libfreetype, libfribidi, libgme, libharfbuzz, libjxl, libkvazaar, liblcevc-dec, libmp3lame, liboapv, libopencore-amrnb, libopencore-amrwb, libopenh264, libopenjpeg, libopenmpt, libopus, libplacebo, librav1e, librist, librsvg, libsnappy, libsoxr, libsrt, libssh, libsvtav1, libtheora, libtwolame, libuavs3d, libvmaf, libvorbis, libvpl, libvpx, libvvenc, libwebp, libxml2, libzimg, libzmq, libzvbi, lv2, lzma, openal, opencl, sdl2, vaapi, vapoursynth, vulkan, zlib.
- **Windows only:** schannel. **Linux only:** libdrm, libpulse, libxcb, openssl, xlib.

Each of these libraries is distributed under its own license by its own project; many of them are BSD-style and ask for their copyright notice to be reproduced with binary distributions. Their texts are available from the corresponding projects and are not repeated here. The macOS FFmpeg libraries are built by the KUI project from the official FFmpeg 9.0.2 release with only VideoToolbox, AudioToolbox, zlib and bzip2 enabled, and contain none of these libraries.

These notices describe copyright licenses only. No patent license is granted for any codec that FFmpeg or the libraries above can decode.
