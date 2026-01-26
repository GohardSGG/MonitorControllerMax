//! Web 静态资源嵌入
//!
//! 使用 rust-embed 将前端文件编译进二进制

use rust_embed::RustEmbed;

/// 嵌入 Web 前端资源
#[derive(RustEmbed)]
#[folder = "../../assets/"]
pub struct Assets;
