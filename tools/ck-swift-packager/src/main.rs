use anyhow::{anyhow, bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{Metadata, MetadataCommand, Package, Target, TargetKind};
use clap::{Parser, ValueEnum};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::BTreeMap;
use uniffi_bindgen::bindings::SwiftBindingGenerator;
use uniffi_bindgen::cargo_metadata::CrateConfigSupplier;

#[derive(Parser, Debug)]
#[command(author, version, about = "Package a UniFFI Rust library into SwiftPM or CocoaPods")]
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
    build_mode: BuildMode,

    /// Library type to package
    #[arg(long, value_enum, default_value = "static")]
    lib_type: LibType,

    /// Output format
    #[arg(long, value_enum, default_value = "spm")]
    format: PackageFormat,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum BuildMode {
    Debug,
    Release,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum LibType {
    Static,
    Dynamic,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum PackageFormat {
    Spm,
    Pod,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let metadata = load_metadata(&args.crate_path)?;
    let package = pick_package(&metadata, args.package.as_deref())?;
    let lib_target = pick_lib_target(package)?;

    let lib_name = args
        .lib_name
        .clone()
        .unwrap_or_else(|| lib_target.name.replace('-', "_"));

    let package_name = args
        .package_name
        .clone()
        .unwrap_or_else(|| package.name.clone());

    let default_xcframework_name = format!("{lib_name}FFI");
    let xcframework_name = args
        .xcframework_name
        .clone()
        .unwrap_or(default_xcframework_name);

    let target_triples = resolve_targets(args.targets)?;

    let output_root = args
        .output
        .clone()
        .unwrap_or_else(|| args.crate_path.join("dist"));
    let output_root = output_root
        .canonicalize()
        .unwrap_or_else(|_| output_root.clone());

    let generated_dir = output_root.join("_generated");
    let package_root = output_root.join(&package_name);

    fs::create_dir_all(&output_root)?;
    fs::create_dir_all(&package_root)?;

    let lib_paths = build_targets(
        &metadata,
        package,
        &lib_name,
        &target_triples,
        args.build_mode,
        args.lib_type,
    )?;

    let lib_paths = coalesce_libraries(&target_triples, &lib_paths, &output_root)?;

    generate_swift_bindings(
        &lib_paths[0],
        &generated_dir,
        &metadata,
    )?;

    let xcframework_path = output_root.join(format!("{xcframework_name}.xcframework"));
    if xcframework_path.exists() {
        fs::remove_dir_all(&xcframework_path).ok();
    }
    create_xcframework(
        &lib_paths,
        &generated_dir,
        &xcframework_path,
    )?;
    patch_xcframework(&xcframework_path, &generated_dir, &xcframework_name)?;

    let package_xcframework = package_root.join(format!("{xcframework_name}.xcframework"));
    if package_xcframework.exists() {
        fs::remove_dir_all(&package_xcframework).ok();
    }
    copy_dir(&xcframework_path, &package_xcframework)?;

    let sources_dir = package_root.join("Sources").join(&package_name);
    if sources_dir.exists() {
        fs::remove_dir_all(&sources_dir).ok();
    }
    fs::create_dir_all(&sources_dir)?;
    copy_generated_sources(&generated_dir, &sources_dir)?;

    match args.format {
        PackageFormat::Spm => {
            write_spm_manifest(&package_root, &package_name, &xcframework_name)?;
        }
        PackageFormat::Pod => {
            write_podspec(&package_root, &package_name, &xcframework_name)?;
        }
    }

    Ok(())
}

fn load_metadata(crate_path: &Path) -> Result<Metadata> {
    let manifest_path = crate_path.join("Cargo.toml");
    if !manifest_path.exists() {
        bail!("Cargo.toml not found at {}", manifest_path.display());
    }
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path(manifest_path);
    let metadata = cmd.exec()?;
    Ok(metadata)
}

fn pick_package<'a>(metadata: &'a Metadata, name: Option<&str>) -> Result<&'a Package> {
    if let Some(name) = name {
        metadata
            .packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("package '{name}' not found in workspace"))
    } else {
        metadata
            .root_package()
            .ok_or_else(|| anyhow!("no root package found; use --package to select one"))
    }
}

fn pick_lib_target<'a>(package: &'a Package) -> Result<&'a Target> {
    package
        .targets
        .iter()
        .find(|t| {
            t.kind.iter().any(|k| {
                matches!(
                    k,
                    TargetKind::Lib
                        | TargetKind::StaticLib
                        | TargetKind::CDyLib
                        | TargetKind::DyLib
                        | TargetKind::RLib
                )
            })
        })
        .ok_or_else(|| anyhow!("no lib target found in {}", package.name))
}

fn resolve_targets(values: Option<Vec<String>>) -> Result<Vec<String>> {
    let values = values.unwrap_or_else(|| vec!["ios".into(), "ios-sim".into()]);
    let mut targets = Vec::new();
    for item in values {
        let triple = match item.as_str() {
            "ios" => "aarch64-apple-ios",
            "ios-sim" => "aarch64-apple-ios-sim",
            "ios-sim-x86_64" => "x86_64-apple-ios",
            "macos" => "aarch64-apple-darwin",
            "macos-x86_64" => "x86_64-apple-darwin",
            other => other,
        };
        targets.push(triple.to_string());
    }
    Ok(targets)
}

fn build_targets(
    metadata: &Metadata,
    package: &Package,
    lib_name: &str,
    targets: &[String],
    build_mode: BuildMode,
    lib_type: LibType,
) -> Result<Vec<PathBuf>> {
    let mut built = Vec::new();
    for target in targets {
        build_target(metadata, package, target, build_mode)?;
        let lib_path = lib_output_path(
            &metadata.target_directory,
            target,
            build_mode,
            lib_name,
            lib_type,
        )?;
        if !lib_path.exists() {
            bail!("library not found at {}", lib_path.display());
        }
        built.push(lib_path);
    }
    Ok(built)
}

fn build_target(
    metadata: &Metadata,
    package: &Package,
    target: &str,
    build_mode: BuildMode,
) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if matches!(build_mode, BuildMode::Release) {
        cmd.arg("--release");
    }
    cmd.args(["--target", target]);
    cmd.arg("--manifest-path");
    cmd.arg(metadata.workspace_root.join("Cargo.toml").as_std_path());
    cmd.args(["-p", package.name.as_str()]);
    let status = cmd.status().context("failed to run cargo build")?;
    if !status.success() {
        bail!("cargo build failed for target {target}");
    }
    Ok(())
}

fn lib_output_path(
    target_dir: &Utf8Path,
    target: &str,
    build_mode: BuildMode,
    lib_name: &str,
    lib_type: LibType,
) -> Result<PathBuf> {
    let profile = match build_mode {
        BuildMode::Debug => "debug",
        BuildMode::Release => "release",
    };
    let file_name = match lib_type {
        LibType::Static => format!("lib{lib_name}.a"),
        LibType::Dynamic => format!("lib{lib_name}.dylib"),
    };
    Ok(target_dir
        .join(target)
        .join(profile)
        .join(file_name)
        .into_std_path_buf())
}

fn generate_swift_bindings(lib_path: &Path, out_dir: &Path, metadata: &Metadata) -> Result<()> {
    let out_dir = Utf8PathBuf::from_path_buf(out_dir.to_path_buf())
        .map_err(|_| anyhow!("output directory contains non-utf8 characters"))?;
    if out_dir.exists() {
        fs::remove_dir_all(out_dir.as_std_path()).ok();
    }
    fs::create_dir_all(out_dir.as_std_path())?;

    let headers = out_dir.join("headers");
    let sources = out_dir.join("sources");
    fs::create_dir_all(headers.as_std_path())?;
    fs::create_dir_all(sources.as_std_path())?;

    let lib_path = Utf8PathBuf::from_path_buf(lib_path.to_path_buf())
        .map_err(|_| anyhow!("library path contains non-utf8 characters"))?;

    let outputs = uniffi_bindgen::library_mode::generate_bindings(
        lib_path.as_ref(),
        None,
        &SwiftBindingGenerator {},
        &CrateConfigSupplier::from(metadata.clone()),
        None,
        out_dir.as_ref(),
        false,
    )?;

    let modulemap_path = headers.join("module.modulemap");
    let mut modulemap = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(modulemap_path.as_std_path())?;

    for output in outputs {
        let crate_name = output.ci.crate_name();
        let swift_src = out_dir.join(format!("{crate_name}.swift"));
        let ffi_name = format!("{crate_name}FFI");
        let ffi_header = out_dir.join(format!("{ffi_name}.h"));
        let ffi_modulemap = out_dir.join(format!("{ffi_name}.modulemap"));

        fs::copy(swift_src.as_std_path(), sources.join(format!("{crate_name}.swift")).as_std_path())?;
        fs::copy(ffi_header.as_std_path(), headers.join(format!("{ffi_name}.h")).as_std_path())?;

        let mut modulemap_part = fs::OpenOptions::new()
            .read(true)
            .open(ffi_modulemap.as_std_path())?;
        std::io::copy(&mut modulemap_part, &mut modulemap)?;
    }

    Ok(())
}

fn create_xcframework(
    lib_paths: &[PathBuf],
    generated_dir: &Path,
    output: &Path,
) -> Result<()> {
    let headers = generated_dir.join("headers");
    let mut cmd = Command::new("xcodebuild");
    cmd.arg("-create-xcframework");
    for lib in lib_paths {
        cmd.arg("-library").arg(lib);
        cmd.arg("-headers").arg(&headers);
    }
    cmd.arg("-output").arg(output);
    let status = cmd.status().context("failed to run xcodebuild")?;
    if !status.success() {
        bail!("xcodebuild -create-xcframework failed");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ApplePlatform {
    Ios,
    IosSim,
    Macos,
}

impl ApplePlatform {
    fn label(self) -> &'static str {
        match self {
            ApplePlatform::Ios => "ios",
            ApplePlatform::IosSim => "ios-sim",
            ApplePlatform::Macos => "macos",
        }
    }
}

fn platform_for_target(target: &str) -> Result<ApplePlatform> {
    match target {
        "aarch64-apple-ios" => Ok(ApplePlatform::Ios),
        "aarch64-apple-ios-sim" => Ok(ApplePlatform::IosSim),
        "x86_64-apple-ios" => Ok(ApplePlatform::IosSim),
        "aarch64-apple-darwin" => Ok(ApplePlatform::Macos),
        "x86_64-apple-darwin" => Ok(ApplePlatform::Macos),
        other => bail!("unsupported apple target: {other}"),
    }
}

fn coalesce_libraries(
    targets: &[String],
    lib_paths: &[PathBuf],
    output_root: &Path,
) -> Result<Vec<PathBuf>> {
    let mut grouped: BTreeMap<ApplePlatform, Vec<PathBuf>> = BTreeMap::new();
    for (target, lib) in targets.iter().zip(lib_paths.iter()) {
        let platform = platform_for_target(target)?;
        grouped.entry(platform).or_default().push(lib.clone());
    }

    let lipo_root = output_root.join("_lipo");
    if lipo_root.exists() {
        fs::remove_dir_all(&lipo_root).ok();
    }
    fs::create_dir_all(&lipo_root)?;

    let mut result = Vec::new();
    for (platform, libs) in grouped {
        if libs.len() == 1 {
            result.push(libs[0].clone());
            continue;
        }

        let file_name = libs[0]
            .file_name()
            .ok_or_else(|| anyhow!("invalid library filename"))?;
        let out_path = lipo_root.join(format!(
            "{}-{}",
            platform.label(),
            file_name.to_string_lossy()
        ));

        let mut cmd = Command::new("lipo");
        cmd.arg("-create");
        for lib in &libs {
            cmd.arg(lib);
        }
        cmd.arg("-output").arg(&out_path);
        let status = cmd.status().context("failed to run lipo")?;
        if !status.success() {
            bail!("lipo failed for platform {}", platform.label());
        }

        result.push(out_path);
    }

    Ok(result)
}

fn patch_xcframework(xcframework: &Path, generated_dir: &Path, name: &str) -> Result<()> {
    let headers_src = generated_dir.join("headers");
    for entry in fs::read_dir(xcframework)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let headers_dir = entry.path().join("Headers");
        if headers_dir.exists() {
            fs::remove_dir_all(&headers_dir).ok();
        }
        let patched_dir = headers_dir.join(name);
        fs::create_dir_all(&patched_dir)?;
        for file in fs::read_dir(&headers_src)? {
            let file = file?;
            let file_path = file.path();
            if file_path.is_file() {
                let file_name = file_path
                    .file_name()
                    .ok_or_else(|| anyhow!("invalid header filename"))?;
                fs::copy(&file_path, patched_dir.join(file_name))?;
            }
        }
    }
    Ok(())
}

fn copy_generated_sources(generated_dir: &Path, dest: &Path) -> Result<()> {
    let sources_dir = generated_dir.join("sources");
    for entry in fs::read_dir(&sources_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("swift")) {
            let filename = path
                .file_name()
                .ok_or_else(|| anyhow!("invalid swift filename"))?;
            fs::copy(&path, dest.join(filename))?;
        }
    }
    Ok(())
}

fn write_spm_manifest(root: &Path, package_name: &str, xcframework_name: &str) -> Result<()> {
    let manifest = format!(
        r#"// swift-tools-version:5.5
import PackageDescription

let package = Package(
    name: "{package_name}",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "{package_name}",
            targets: ["{package_name}"]
        )
    ],
    targets: [
        .binaryTarget(
            name: "{xcframework_name}",
            path: "./{xcframework_name}.xcframework"
        ),
        .target(
            name: "{package_name}",
            dependencies: [
                .target(name: "{xcframework_name}")
            ]
        )
    ]
)
"#
    );
    fs::write(root.join("Package.swift"), manifest)?;
    Ok(())
}

fn write_podspec(root: &Path, package_name: &str, xcframework_name: &str) -> Result<()> {
    let podspec = format!(
        r#"Pod::Spec.new do |s|
  s.name = "{package_name}"
  s.version = "0.1.0"
  s.summary = "Generated UniFFI bindings for {package_name}"
  s.license = {{ :type => "Proprietary" }}
  s.authors = {{ "Cross-Kit" => "dev@cross-kit.local" }}
  s.homepage = "https://example.invalid"
  s.source = {{ :path => "." }}
  s.vendored_frameworks = "{xcframework_name}.xcframework"
  s.source_files = "Sources/{package_name}/**/*.swift"
  s.swift_version = "5.9"
end
"#
    );
    fs::write(root.join(format!("{package_name}.podspec")), podspec)?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
