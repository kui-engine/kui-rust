//! KUI 冻结 C ABI 的机械生成声明。
//!
//! 本 crate 覆盖冻结清单内的全部 product-major-3 导出；普通 consumer 不需要 bindgen
//! 或 libclang。声明由仓库的 `scripts/generate_rust_bindings.py` 更新。

#![no_std]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

include!("bindings.rs");
