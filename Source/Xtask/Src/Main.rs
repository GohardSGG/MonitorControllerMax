#![allow(non_snake_case)]

use anyhow::{anyhow, Context};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

// ---------------------------------------------------------------------------
// Build Variant
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildVariant {
    Dev,
    Release,
    Production,
}

impl BuildVariant {
    fn suffix(self) -> &'static str {
        match self {
            BuildVariant::Dev => "dev",
            BuildVariant::Release => "release",
            BuildVariant::Production => "production",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            BuildVariant::Dev => "Dev",
            BuildVariant::Release => "Release",
            BuildVariant::Production => "Production",
        }
    }

    fn needs_release_flag(self) -> bool {
        matches!(self, BuildVariant::Release | BuildVariant::Production)
    }

    fn is_production(self) -> bool {
        matches!(self, BuildVariant::Production)
    }
}

fn parse_build_variant(args: &[String]) -> anyhow::Result<BuildVariant> {
    let has_dev = args.iter().any(|arg| arg == "--dev");
    let has_release = args.iter().any(|arg| arg == "--release");
    let has_production = args.iter().any(|arg| arg == "--production");

    if has_production {
        if has_dev {
            return Err(anyhow!("--dev and --production cannot be used together"));
        }
        return Ok(BuildVariant::Production);
    }

    if has_dev {
        if has_release {
            return Err(anyhow!("--dev and --release cannot be used together"));
        }
        return Ok(BuildVariant::Dev);
    }

    if has_release {
        Ok(BuildVariant::Release)
    } else {
        Ok(BuildVariant::Dev)
    }
}

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------

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

    #[allow(unreachable_code)]
    "Unknown"
}

fn get_build_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn format_archive_dir_name(platform_label: &str, variant: BuildVariant) -> String {
    format!(
        "MCM_{}_v{}-{}",
        platform_label,
        get_build_version(),
        variant.display_name()
    )
}

// ---------------------------------------------------------------------------
// Build flow helpers
// ---------------------------------------------------------------------------

fn clean_default_bundled_dir(workspace_root: &Path) -> anyhow::Result<()> {
    let bundled_dir = workspace_root.join("target/bundled");
    if bundled_dir.exists() {
        println!("[Pre-Clean] Removing target/bundled to avoid platform conflicts...");
        fs::remove_dir_all(&bundled_dir).context("Failed to remove target/bundled directory")?;
    }
    Ok(())
}

fn prepare_bundle_args(
    raw_args: &[String],
    should_copy: bool,
    variant: BuildVariant,
) -> Vec<String> {
    let mut forwarded: Vec<String> = raw_args
        .iter()
        .skip(1)
        .filter(|arg| *arg != "--dev" && *arg != "--production")
        .cloned()
        .collect();

    if should_copy {
        if forwarded.is_empty() || forwarded[0].starts_with('-') {
            forwarded.insert(0, "bundle".to_string());
        }

        if !forwarded.iter().any(|arg| arg == "monitor_controller_max") {
            if forwarded
                .first()
                .map(|cmd| cmd == "bundle")
                .unwrap_or(false)
            {
                forwarded.insert(1, "monitor_controller_max".to_string());
            } else {
                forwarded.insert(0, "monitor_controller_max".to_string());
            }
        }
    }

    if variant.needs_release_flag() && !forwarded.iter().any(|arg| arg == "--release") {
        forwarded.push("--release".to_string());
    }

    if variant.is_production() {
        let has_production_feature = forwarded.windows(2).any(|pair| {
            pair[0] == "--features" && pair[1].contains("monitor_controller_max/production")
        });

        if !has_production_feature {
            forwarded.push("--features".to_string());
            forwarded.push("monitor_controller_max/production".to_string());
        }
    }

    forwarded
}

fn move_to_platform_dir(
    workspace_root: &Path,
    variant: BuildVariant,
) -> anyhow::Result<PathBuf> {
    let src_dir = workspace_root.join("target/bundled");
    let dst_dir = workspace_root.join("target").join(format!(
        "bundled_{}-{}",
        get_platform_bundle_dir(),
        variant.suffix()
    ));

    if !src_dir.exists() {
        return Err(anyhow!("target/bundled not found after build"));
    }

    if dst_dir.exists() {
        fs::remove_dir_all(&dst_dir)?;
    }
    fs::create_dir_all(&dst_dir)?;

    for entry in fs::read_dir(&src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst_dir.join(entry.file_name());

        if dst_path.exists() {
            if dst_path.is_dir() {
                fs::remove_dir_all(&dst_path)?;
            } else {
                fs::remove_file(&dst_path)?;
            }
        }

        fs::rename(&src_path, &dst_path)
            .with_context(|| format!("Failed to move {:?} to {:?}", src_path, dst_path))?;
    }

    let _ = fs::remove_dir(&src_dir);
    println!("[Platform Bundle] Moved to: {}", dst_dir.display());
    Ok(dst_dir)
}

fn find_first_bundle_with_ext(bundled_dir: &Path, ext: &str) -> anyhow::Result<PathBuf> {
    let entry = fs::read_dir(bundled_dir)
        .context("Failed to read bundled directory")?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().extension().map_or(false, |e| e == ext))
        .with_context(|| format!("No .{} bundle found", ext))?;

    Ok(entry.path())
}

fn find_optional_bundle_with_ext(bundled_dir: &Path, ext: &str) -> Option<PathBuf> {
    fs::read_dir(bundled_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().extension().map_or(false, |e| e == ext))
        .map(|entry| entry.path())
}

// ---------------------------------------------------------------------------
// run_bundle_build — the core build + deploy pipeline
// ---------------------------------------------------------------------------

fn run_bundle_build(
    raw_args: &[String],
    variant: BuildVariant,
    workspace_root: &Path,
    deploy_after_build: bool,
) -> anyhow::Result<PathBuf> {
    clean_default_bundled_dir(workspace_root)?;

    let forwarded_args = prepare_bundle_args(raw_args, true, variant);
    nih_plug_xtask::main_with_args("xtask", forwarded_args.into_iter())?;

    std::thread::sleep(std::time::Duration::from_millis(500));
    let platform_dir = move_to_platform_dir(workspace_root, variant)?;

    if deploy_after_build {
        post_build_copy_mode(variant, &platform_dir)?;
    }

    Ok(platform_dir)
}

// ---------------------------------------------------------------------------
// Post-build deploy
// ---------------------------------------------------------------------------

fn post_build_copy_mode(
    variant: BuildVariant,
    bundled_dir: &Path,
) -> anyhow::Result<()> {
    println!(
        "\n[Auto-Copy] Starting post-build copy for {} mode...",
        variant.display_name()
    );
    let workspace_root = env::current_dir()?;

    if !bundled_dir.exists() {
        return Err(anyhow!("bundled directory not found"));
    }

    let src_path = find_first_bundle_with_ext(bundled_dir, "vst3")?;
    let dir_name = src_path.file_name().context("Invalid source filename")?;

    // Archive to Build/<label>/
    let archive_dir_name = format_archive_dir_name(get_platform_bundle_dir(), variant);
    let internal_build_dir = workspace_root.join("Build").join(&archive_dir_name);
    if internal_build_dir.exists() {
        fs::remove_dir_all(&internal_build_dir)?;
    }
    fs::create_dir_all(&internal_build_dir)?;

    let archived_bundle_path = internal_build_dir.join(dir_name);
    copy_dir_recursive(&src_path, &archived_bundle_path)?;
    println!(
        "[Auto-Copy] Archived to: {}",
        archived_bundle_path.display()
    );

    // Also archive CLAP if present
    if let Some(clap_src) = find_optional_bundle_with_ext(bundled_dir, "clap") {
        if let Some(clap_name) = clap_src.file_name() {
            let clap_dst = internal_build_dir.join(clap_name);
            if let Err(error) = copy_path(&clap_src, &clap_dst) {
                println!(
                    "[Auto-Copy] Warning: Failed to archive CLAP: {}",
                    error
                );
            } else {
                println!("[Auto-Copy] Archived CLAP to: {}", clap_dst.display());
            }
        }
    }

    // Deploy to plugin directories
    #[cfg(target_os = "windows")]
    {
        let deploy_targets: Vec<(PathBuf, bool)> = match variant {
            BuildVariant::Dev => vec![(Path::new(r"C:\Plugins\VST Dev").to_path_buf(), false)],
            BuildVariant::Release => vec![
                (Path::new(r"C:\Plugins\VST Dev").to_path_buf(), false),
                (
                    Path::new(r"C:\Program Files\Common Files\VST3").to_path_buf(),
                    true,
                ),
            ],
            BuildVariant::Production => {
                vec![(
                    Path::new(r"C:\Program Files\Common Files\VST3").to_path_buf(),
                    true,
                )]
            }
        };

        for (deploy_dir, best_effort) in deploy_targets {
            if !deploy_dir.exists() {
                if best_effort {
                    let _ = fs::create_dir_all(&deploy_dir);
                } else {
                    fs::create_dir_all(&deploy_dir)?;
                }
            }

            let deployed_bundle_path = deploy_dir.join(dir_name);
            let copy_result = copy_dir_recursive(&src_path, &deployed_bundle_path);
            if best_effort {
                if let Err(error) = copy_result {
                    println!(
                        "[Auto-Copy] Warning: Failed to deploy to {} (permission issue?): {:?}",
                        deploy_dir.display(),
                        error
                    );
                    continue;
                }
            } else {
                copy_result?;
            }

            println!(
                "[Auto-Copy] Deployed to: {}",
                deployed_bundle_path.display()
            );
        }

        println!("[Auto-Copy] {} mode complete!\n", variant.display_name());
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let deploy_dirs: Vec<PathBuf> = if matches!(variant, BuildVariant::Dev) {
            vec![Path::new("/Plugins/VST Dev").to_path_buf()]
        } else {
            let home = env::var("HOME").context("HOME is not set")?;
            vec![
                Path::new("/Library/Audio/Plug-Ins/VST3").to_path_buf(),
                Path::new(&home).join("Library/Audio/Plug-Ins/VST3"),
            ]
        };

        if matches!(variant, BuildVariant::Dev) {
            let system_bundle = Path::new("/Library/Audio/Plug-Ins/VST3").join(dir_name);
            if system_bundle.exists() {
                println!(
                    "[Auto-Copy] Notice: System bundle also exists at {}.",
                    system_bundle.display()
                );
            }
        }

        for deploy_dir in deploy_dirs {
            fs::create_dir_all(&deploy_dir).with_context(|| {
                format!(
                    "Failed to create deploy directory: {}",
                    deploy_dir.display()
                )
            })?;

            let deployed_bundle_path = deploy_dir.join(dir_name);
            copy_dir_recursive(&src_path, &deployed_bundle_path).with_context(|| {
                format!(
                    "Failed to deploy bundle to {}",
                    deployed_bundle_path.display()
                )
            })?;
            println!(
                "[Auto-Copy] Deployed to: {}",
                deployed_bundle_path.display()
            );
            sign_vst3_bundle_macos(&deployed_bundle_path)?;
        }

        println!("[Auto-Copy] {} mode complete!\n", variant.display_name());
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        println!("[Auto-Copy] {} mode complete!\n", variant.display_name());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// File system helpers
// ---------------------------------------------------------------------------

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }

    Ok(())
}

fn copy_path(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst).with_context(|| {
            format!(
                "Failed to copy file from {} to {}",
                src.display(),
                dst.display()
            )
        })?;
        Ok(())
    }
}

/// macOS Ad-hoc signing
#[cfg(target_os = "macos")]
fn sign_vst3_bundle_macos(bundle_path: &Path) -> anyhow::Result<()> {
    println!("[Auto-Copy] Signing VST3 bundle for macOS (Ad-hoc)...");

    let status = Command::new("codesign")
        .args([
            "--force",
            "--deep",
            "--sign",
            "-",
            bundle_path.to_str().unwrap(),
        ])
        .status()
        .context("Failed to execute codesign")?;

    if status.success() {
        println!("[Auto-Copy] Signing successful.");
    } else {
        println!("[Auto-Copy] Warning: Signing failed.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let should_bundle = args.iter().any(|arg| arg == "bundle");
    let variant = parse_build_variant(&args)?;
    let workspace_root = env::current_dir()?;

    println!("[Platform] Target: {}", get_platform_bundle_dir());
    println!("[Mode] Build variant: {}", variant.display_name());
    println!("[Version] Package version: {}", get_build_version());

    if should_bundle {
        println!("[Command] bundle");
        println!(
            "[Archive Layout] Build/{}/",
            format_archive_dir_name(get_platform_bundle_dir(), variant)
        );
        let _platform_dir = run_bundle_build(&args, variant, &workspace_root, true)?;
        Ok(())
    } else {
        // Passthrough to nih_plug_xtask for any non-bundle commands
        let forwarded_args = prepare_bundle_args(&args, false, variant);
        nih_plug_xtask::main_with_args("xtask", forwarded_args.into_iter())
    }
}
