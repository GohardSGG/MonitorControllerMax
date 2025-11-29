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
    // 如果 nih_plug_xtask 内部有 exit 调用，这里的代码确实不会执行。
    // 但是，我们可以利用 Drop trait 或者 panic hook 来做最后的挣扎？不，那太复杂了。
    // 
    // 关键点：cargo xtask 其实是 cargo run -p xtask。
    // 如果 nih_plug_xtask::main() 返回了，那我们就赢了。
    // 如果它没返回，说明它调用了 exit。
    
    // 让我们尝试先打印一条日志，看看是不是真的进入了 main
    // println!("[XTask Wrapper] Starting...");

    let result = nih_plug_xtask::main();

    // 3. 无论结果如何，只要构建成功了（通过检查文件存在与否判断），我们就尝试拷贝
    if should_copy {
        // 给文件系统一点喘息时间
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // 尝试执行拷贝，忽略之前的 result 错误（因为有时 help 也会返回 error）
        // 但我们只关心文件是否生成
        if let Err(e) = post_build_copy(&args) {
            // 如果拷贝失败，打印错误，但不一定让整个进程失败（取决于需求）
            eprintln!("[Auto-Copy] Copy failed: {:?}", e);
        }
    }

    result
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
        return Ok(());
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
