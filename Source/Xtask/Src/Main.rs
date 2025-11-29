#![allow(non_snake_case)]

use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // 1. 检查是否是 bundle 命令
    let args: Vec<String> = env::args().collect();
    let should_copy = args.iter().any(|arg| arg == "bundle");

    // 2. 执行标准的 nih_plug xtask 构建流程
    // 注意：我们必须确保 nih_plug_xtask 执行完毕后再执行我们的拷贝逻辑
    
    // 关键点：cargo xtask 其实是 cargo run -p xtask。
    let result = nih_plug_xtask::main();

    // 3. 只有构建成功才尝试拷贝
    // 如果 result 是 Err，说明构建/打包失败，直接返回错误，不执行拷贝
    if let Err(e) = result {
        return Err(e);
    }

    if should_copy {
        // 给文件系统一点喘息时间
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // 尝试执行拷贝
        // 使用 ? 操作符：如果 post_build_copy 失败，main 函数会返回 Err，
        // 从而导致非零的退出码，满足“显得是失败的”这一要求。
        post_build_copy(&args)?;
    }

    Ok(())
}

/// 构建后自动拷贝任务
fn post_build_copy(args: &[String]) -> anyhow::Result<()> {
    // 判断构建模式
    let is_release = args.iter().any(|arg| arg == "--release");
    let profile = if is_release { "Release" } else { "Debug" };
    
    println!("\n[Auto-Copy] Starting post-build copy for {} profile...", profile);

    // 获取 workspace 根目录 (假设在项目根目录下运行 cargo xtask)
    let workspace_root = env::current_dir()?;
    let bundled_dir = workspace_root.join("target/bundled");

    // 查找生成的 VST3 目录
    // 我们不硬编码文件名，而是查找第一个 .vst3 目录，这样更灵活
    if !bundled_dir.exists() {
        println!("[Auto-Copy] Warning: target/bundled directory not found. Skipping copy.");
        // 如果目录不存在，虽然可能是因为没有生成，但 nih_plug_xtask 成功了，
        // 可能是没运行 bundle 命令？或者清理了？
        // 这里返回 Ok 还是 Err 取决于严格程度。如果 nih_plug 成功但没有输出，可能是配置问题。
        // 但既然 should_copy 为 true，这里应该是期望有输出的。
        return Err(anyhow::anyhow!("target/bundled not found after successful build"));
    }

    let vst3_entry = fs::read_dir(&bundled_dir)
        .context("Failed to read target/bundled directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path().extension().map_or(false, |ext| ext == "vst3")
        })
        .context("No .vst3 bundle found in target/bundled. Did the build succeed?")?;

    let src_path = vst3_entry.path();
    let dir_name = src_path.file_name().context("Invalid source filename")?;

    // ---------------------------------------------------------
    // 目标 1: 项目内部归档目录 (Build/Debug 或 Build/Release)
    // ---------------------------------------------------------
    let internal_build_dir = workspace_root.join("Build").join(profile);
    
    // 确保目录存在
    if !internal_build_dir.exists() {
        fs::create_dir_all(&internal_build_dir)?;
    }
    
    let dest_path_1 = internal_build_dir.join(dir_name);
    
    // 执行递归拷贝
    copy_dir_recursive(&src_path, &dest_path_1)?;
    println!("[Auto-Copy] Archived to: {}", dest_path_1.display());

    // ---------------------------------------------------------
    // 目标 2: 系统 VST 开发目录 (C:\Plugins\VST Dev)
    // ---------------------------------------------------------
    let external_dev_dir = Path::new(r"C:\Plugins\VST Dev");
    
    // 如果目录不存在，自动创建
    if !external_dev_dir.exists() {
        fs::create_dir_all(external_dev_dir)?;
    }
    
    let dest_path_2 = external_dev_dir.join(dir_name);
    
    // 执行递归拷贝
    copy_dir_recursive(&src_path, &dest_path_2)?;
    println!("[Auto-Copy] Deployed to: {}", dest_path_2.display());

    println!("[Auto-Copy] Success!\n");
    Ok(())
}

/// 递归拷贝目录的辅助函数 (类似 cp -r)
/// Windows 下 std::fs::copy 不支持目录，需要手动递归
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    // 如果目标目录不存在，创建它
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            // 如果是文件，直接拷贝 (覆盖模式)
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
