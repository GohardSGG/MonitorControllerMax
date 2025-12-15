#![allow(non_snake_case)]

use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use anyhow::Context;

/// 获取平台特定的 bundled 目录名称
/// Windows: Windows_X86-64
/// macOS ARM: MacOS_AArch64
/// macOS Intel: MacOS_X86-64
/// Linux: Linux_X86-64
fn get_platform_bundle_dir() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "Windows_X86-64";

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    return "Windows_X86";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "MacOS_AArch64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "MacOS_X86-64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "Linux_X86-64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "Linux_AArch64";

    // Fallback
    #[allow(unreachable_code)]
    "Unknown"
}

/// 清理默认的 bundled 目录（解决 Mac/Windows 格式冲突）
fn clean_default_bundled_dir(workspace_root: &Path) -> anyhow::Result<()> {
    let bundled_dir = workspace_root.join("target/bundled");
    if bundled_dir.exists() {
        println!("[Pre-Clean] Removing target/bundled to avoid platform conflicts...");
        fs::remove_dir_all(&bundled_dir)
            .context("Failed to remove target/bundled directory")?;
    }
    Ok(())
}

/// 将 bundled 目录内容移动到平台特定目录
fn move_to_platform_dir(workspace_root: &Path) -> anyhow::Result<PathBuf> {
    let src_dir = workspace_root.join("target/bundled");
    let platform_dir_name = get_platform_bundle_dir();
    let dst_dir = workspace_root.join("target/bundled").with_file_name(format!("bundled_{}", platform_dir_name));

    if !src_dir.exists() {
        return Err(anyhow::anyhow!("target/bundled not found after build"));
    }

    // 确保目标目录存在
    if dst_dir.exists() {
        fs::remove_dir_all(&dst_dir)?;
    }
    fs::create_dir_all(&dst_dir)?;

    // 移动所有内容
    for entry in fs::read_dir(&src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst_dir.join(entry.file_name());

        // 如果目标已存在，先删除
        if dst_path.exists() {
            if dst_path.is_dir() {
                fs::remove_dir_all(&dst_path)?;
            } else {
                fs::remove_file(&dst_path)?;
            }
        }

        // 移动（重命名）
        fs::rename(&src_path, &dst_path)
            .with_context(|| format!("Failed to move {:?} to {:?}", src_path, dst_path))?;
    }

    // 删除空的 src_dir
    let _ = fs::remove_dir(&src_dir);

    println!("[Platform Bundle] Moved to: {}", dst_dir.display());
    Ok(dst_dir)
}

fn main() -> anyhow::Result<()> {
    // 1. 检查命令参数
    let args: Vec<String> = env::args().collect();
    let should_copy = args.iter().any(|arg| arg == "bundle");
    let is_production = args.iter().any(|arg| arg == "--production");

    // 获取 workspace 根目录
    let workspace_root = env::current_dir()?;

    // 2. 如果是 production 模式，需要设置环境变量传递 feature
    if is_production && should_copy {
        println!("\n[Production Build] Starting production build with --features production...");
        println!("[Platform] Target: {}", get_platform_bundle_dir());

        // 清理默认 bundled 目录（避免 Mac/Windows 格式冲突）
        clean_default_bundled_dir(&workspace_root)?;

        // 方法：直接设置 CARGO_FEATURE_PRODUCTION 环境变量
        // 然后调用标准的 xtask bundle 流程
        env::set_var("CARGO_FEATURE_PRODUCTION", "1");

        // 调用 nih_plug_xtask（它会使用当前环境变量）
        // 但这不会工作，因为 nih_plug_xtask 内部调用 cargo build 不会传递这个变量

        // 更好的方法：直接使用 cargo build + 手动 bundle
        // Step 1: 带 feature 构建
        println!("[Production Build] Step 1: Building with production feature...");
        let build_status = Command::new("cargo")
            .args([
                "build", "-p", "monitor_controller_max",
                "--lib",
                "--release",
                "--features", "monitor_controller_max/production"
            ])
            .current_dir(&workspace_root)
            .status()
            .context("Failed to execute cargo build")?;

        if !build_status.success() {
            return Err(anyhow::anyhow!("Production build failed"));
        }

        // Step 2: 调用 nih_plug_xtask bundle（它会使用已编译的库）
        println!("[Production Build] Step 2: Bundling...");

        // 过滤掉 --production 参数，并确保添加 --release
        // 跳过第一个参数（程序路径），从第二个开始
        let mut filtered_args: Vec<String> = args.iter()
            .skip(1)  // 跳过程序路径
            .filter(|arg| *arg != "--production")
            .cloned()
            .collect();

        // 确保有 --release 标志（production 模式必须是 release 构建）
        if !filtered_args.iter().any(|a| a == "--release") {
            filtered_args.push("--release".to_string());
        }

        // 调用 nih_plug_xtask bundle（它会使用已编译的库）
        // main_with_args 需要 command_name 和 args
        let result = nih_plug_xtask::main_with_args(
            "xtask",
            filtered_args.into_iter()
        );

        if let Err(e) = result {
            return Err(e);
        }

        // 给文件系统一点喘息时间
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 移动到平台特定目录
        let platform_dir = move_to_platform_dir(&workspace_root)?;

        // 执行 production 拷贝（部署到系统 VST3 目录）
        post_build_copy_production(&platform_dir)?;

        return Ok(());
    }

    // 3. 标准构建流程（非 production）
    println!("[Platform] Target: {}", get_platform_bundle_dir());

    // 清理默认 bundled 目录（避免 Mac/Windows 格式冲突）
    if should_copy {
        clean_default_bundled_dir(&workspace_root)?;
    }

    let result = nih_plug_xtask::main();

    // 4. 只有构建成功才尝试拷贝
    if let Err(e) = result {
        return Err(e);
    }

    if should_copy {
        // 给文件系统一点喘息时间
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 移动到平台特定目录
        let platform_dir = move_to_platform_dir(&workspace_root)?;

        // 尝试执行拷贝
        post_build_copy(&args, &platform_dir)?;
    }

    Ok(())
}

/// 构建后自动拷贝任务（开发模式）
fn post_build_copy(args: &[String], bundled_dir: &Path) -> anyhow::Result<()> {
    // 判断构建模式
    let is_release = args.iter().any(|arg| arg == "--release");
    let profile = if is_release { "Release" } else { "Debug" };

    println!("\n[Auto-Copy] Starting post-build copy for {} profile...", profile);

    // 获取 workspace 根目录 (假设在项目根目录下运行 cargo xtask)
    let workspace_root = env::current_dir()?;

    // 查找生成的 VST3 目录
    if !bundled_dir.exists() {
        println!("[Auto-Copy] Warning: bundled directory not found. Skipping copy.");
        return Err(anyhow::anyhow!("bundled directory not found after successful build"));
    }

    let vst3_entry = fs::read_dir(bundled_dir)
        .context("Failed to read bundled directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path().extension().map_or(false, |ext| ext == "vst3")
        })
        .context("No .vst3 bundle found in bundled directory. Did the build succeed?")?;

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
    // 目标 2: 系统 VST 开发目录
    // Windows: C:\Plugins\VST Dev
    // macOS: ~/Library/Audio/Plug-Ins/VST3
    // ---------------------------------------------------------
    #[cfg(target_os = "windows")]
    let external_dev_dir = Path::new(r"C:\Plugins\VST Dev").to_path_buf();

    #[cfg(target_os = "macos")]
    let external_dev_dir = {
        let home = env::var("HOME").context("Failed to get HOME env var")?;
        Path::new(&home).join("Library/Audio/Plug-Ins/VST3")
    };
    
    // 如果目录不存在，自动创建
    if !external_dev_dir.exists() {
        fs::create_dir_all(&external_dev_dir)?;
    }

    let dest_path_2 = external_dev_dir.join(dir_name);

    // 执行递归拷贝
    copy_dir_recursive(&src_path, &dest_path_2)?;
    println!("[Auto-Copy] Deployed to: {}", dest_path_2.display());

    // macOS: 执行代码签名
    #[cfg(target_os = "macos")]
    sign_vst3_bundle_macos(&dest_path_2)?;

    println!("[Auto-Copy] Success!\n");
    Ok(())
}

/// 构建后自动拷贝任务（生产模式）
fn post_build_copy_production(bundled_dir: &Path) -> anyhow::Result<()> {
    println!("\n[Auto-Copy] Starting post-build copy for Production profile...");

    // 获取 workspace 根目录
    let workspace_root = env::current_dir()?;

    // 查找生成的 VST3 目录
    if !bundled_dir.exists() {
        println!("[Auto-Copy] Warning: bundled directory not found. Skipping copy.");
        return Err(anyhow::anyhow!("bundled directory not found after successful build"));
    }

    let vst3_entry = fs::read_dir(bundled_dir)
        .context("Failed to read bundled directory")?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path().extension().map_or(false, |ext| ext == "vst3")
        })
        .context("No .vst3 bundle found in bundled directory. Did the build succeed?")?;

    let src_path = vst3_entry.path();
    let dir_name = src_path.file_name().context("Invalid source filename")?;

    // ---------------------------------------------------------
    // 目标 1: 项目内部归档目录 (Build/Production)
    // ---------------------------------------------------------
    let internal_build_dir = workspace_root.join("Build").join("Production");

    // 确保目录存在
    if !internal_build_dir.exists() {
        fs::create_dir_all(&internal_build_dir)?;
    }

    let dest_path_1 = internal_build_dir.join(dir_name);

    // 执行递归拷贝
    copy_dir_recursive(&src_path, &dest_path_1)?;
    println!("[Auto-Copy] Archived to: {}", dest_path_1.display());

    // ---------------------------------------------------------
    // 目标 2: 系统 VST3 目录
    // Windows: C:\Program Files\Common Files\VST3
    // macOS: /Library/Audio/Plug-Ins/VST3 (Root)
    // ---------------------------------------------------------
    #[cfg(target_os = "windows")]
    let system_vst3_dir = Path::new(r"C:\Program Files\Common Files\VST3").to_path_buf();

    #[cfg(target_os = "macos")]
    let system_vst3_dir = Path::new("/Library/Audio/Plug-Ins/VST3").to_path_buf();

    // 如果目录不存在，自动创建（需要管理员权限）
    if !system_vst3_dir.exists() {
        fs::create_dir_all(&system_vst3_dir)
            .context("Failed to create system VST3 directory. Run as Administrator/Root?")?;
    }

    let dest_path_2 = system_vst3_dir.join(dir_name);

    // 执行递归拷贝
    copy_dir_recursive(&src_path, &dest_path_2)
        .context("Failed to copy to system VST3 directory. Run as Administrator/Root?")?;
    println!("[Auto-Copy] Deployed to: {}", dest_path_2.display());

    // macOS: 执行代码签名
    #[cfg(target_os = "macos")]
    sign_vst3_bundle_macos(&dest_path_2)?;

    println!("[Auto-Copy] Production build complete!\n");
    Ok(())
}

/// 递归拷贝目录的辅助函数 (类似 cp -r)
/// Windows 下 std::fs::copy 不支持目录，需要手动递归
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    // 如果目标目录已存在，先删除（确保干净的拷贝）
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }

    // 创建目标目录
    fs::create_dir_all(dst)?;

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

/// macOS Ad-hoc 签名
#[cfg(target_os = "macos")]
fn sign_vst3_bundle_macos(bundle_path: &Path) -> anyhow::Result<()> {
    println!("[Auto-Copy] Signing VST3 bundle for macOS (Ad-hoc)...");
    
    let status = Command::new("codesign")
        .args([
            "--force",
            "--deep",
            "--sign", "-",
            bundle_path.to_str().unwrap()
        ])
        .status()
        .context("Failed to execute codesign")?;

    if status.success() {
        println!("[Auto-Copy] Signing successful.");
    } else {
        println!("[Auto-Copy] Warning: Signing failed. Logic Pro might not load the plugin.");
    }

    Ok(())
}
