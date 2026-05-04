use anyhow::Result;
use clap::{Parser, ValueEnum};
use cross_kit_packager_ios::{BuildMode, IosPackageOptions, LibType, PackageFormat};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Compatibility wrapper for `cross-kit ios package`"
)]
struct Args {
    /// Path to the Rust crate (directory containing Cargo.toml)
    #[arg(long, default_value = ".")]
    crate_path: PathBuf,

    /// Package name (SwiftPM/CocoaPods)
    #[arg(long)]
    package_name: Option<String>,

    /// Rust package name (workspace-aware)
    #[arg(long)]
    package: Option<String>,

    /// Rust library name (defaults to lib target name)
    #[arg(long)]
    lib_name: Option<String>,

    /// Output directory for the packaged artifacts
    #[arg(long)]
    output: Option<PathBuf>,

    /// Name for the generated XCFramework
    #[arg(long)]
    xcframework_name: Option<String>,

    /// Build targets (comma-separated). Accepts aliases: ios, ios-sim, ios-sim-x86_64, macos, macos-x86_64
    #[arg(long, value_delimiter = ',')]
    targets: Option<Vec<String>>,

    /// Build mode
    #[arg(long, value_enum, default_value = "release")]
    build_mode: CliBuildMode,

    /// Library type to package
    #[arg(long, value_enum, default_value = "static")]
    lib_type: CliLibType,

    /// Output format
    #[arg(long, value_enum, default_value = "spm")]
    format: CliPackageFormat,

    /// Generate Swift bridges emitted by ck_vm_bridge metadata
    #[arg(long)]
    swift_bridges: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum CliBuildMode {
    Debug,
    Release,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum CliLibType {
    Static,
    Dynamic,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum CliPackageFormat {
    Spm,
    Pod,
}

fn main() -> Result<()> {
    let args = Args::parse();
    cross_kit_packager_ios::package_ios(&IosPackageOptions {
        crate_path: args.crate_path,
        package_name: args.package_name,
        package: args.package,
        lib_name: args.lib_name,
        output: args.output,
        xcframework_name: args.xcframework_name,
        targets: args.targets,
        build_mode: match args.build_mode {
            CliBuildMode::Debug => BuildMode::Debug,
            CliBuildMode::Release => BuildMode::Release,
        },
        lib_type: match args.lib_type {
            CliLibType::Static => LibType::Static,
            CliLibType::Dynamic => LibType::Dynamic,
        },
        format: match args.format {
            CliPackageFormat::Spm => PackageFormat::Spm,
            CliPackageFormat::Pod => PackageFormat::Pod,
        },
        swift_bridges: args.swift_bridges,
        metadata_bin: "ck_vm_metadata".to_string(),
    })?;
    Ok(())
}
