use anyhow::{Context, Result, anyhow, bail};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{Metadata, MetadataCommand, Package, Target, TargetKind};
use cross_kit_core::{BindingsConfig, VmMetadata};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use uniffi_bindgen::bindings::SwiftBindingGenerator;
use uniffi_bindgen::cargo_metadata::CrateConfigSupplier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IosPackageOptions {
    pub crate_path: PathBuf,
    pub package_name: Option<String>,
    pub package: Option<String>,
    pub lib_name: Option<String>,
    pub output: Option<PathBuf>,
    pub xcframework_name: Option<String>,
    pub targets: Option<Vec<String>>,
    pub build_mode: BuildMode,
    pub lib_type: LibType,
    pub format: PackageFormat,
    pub swift_bridges: bool,
    pub metadata_bin: String,
    pub bindings: Option<BindingsConfig>,
}

impl Default for IosPackageOptions {
    fn default() -> Self {
        Self {
            crate_path: PathBuf::from("."),
            package_name: None,
            package: None,
            lib_name: None,
            output: None,
            xcframework_name: None,
            targets: None,
            build_mode: BuildMode::Release,
            lib_type: LibType::Static,
            format: PackageFormat::Spm,
            swift_bridges: false,
            metadata_bin: "ck_vm_metadata".to_string(),
            bindings: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IosPackageReport {
    pub package_root: PathBuf,
    pub xcframework_path: PathBuf,
    pub package_name: String,
    pub lib_name: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

impl FromStr for BuildMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "debug" => Ok(Self::Debug),
            "release" => Ok(Self::Release),
            other => bail!("unsupported build mode '{other}'; expected 'debug' or 'release'"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibType {
    Static,
    Dynamic,
}

impl FromStr for LibType {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "static" => Ok(Self::Static),
            "dynamic" => Ok(Self::Dynamic),
            other => bail!("unsupported iOS lib type '{other}'; expected 'static' or 'dynamic'"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    Spm,
    Pod,
}

impl FromStr for PackageFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "spm" => Ok(Self::Spm),
            "pod" => Ok(Self::Pod),
            other => bail!("unsupported iOS package format '{other}'; expected 'spm' or 'pod'"),
        }
    }
}

pub fn package_ios(options: &IosPackageOptions) -> Result<IosPackageReport> {
    let metadata = load_metadata(&options.crate_path)?;
    let package = pick_package(&metadata, options.package.as_deref())?;
    let lib_target = pick_lib_target(package)?;

    let lib_name = options
        .lib_name
        .clone()
        .unwrap_or_else(|| lib_target.name.replace('-', "_"));

    let package_name = options
        .package_name
        .clone()
        .unwrap_or_else(|| package.name.clone());

    let default_xcframework_name = format!("{lib_name}FFI");
    let xcframework_name = options
        .xcframework_name
        .clone()
        .unwrap_or(default_xcframework_name);

    let target_triples = resolve_targets(options.targets.clone())?;

    let output_root = options
        .output
        .clone()
        .unwrap_or_else(|| options.crate_path.join("dist"));
    let output_root = output_root
        .canonicalize()
        .unwrap_or_else(|_| output_root.clone());

    let generated_dir = output_root.join("_generated");
    let package_root = output_root.join(&package_name);
    let staged_package_root = output_root.join(format!("._{package_name}.next"));

    fs::create_dir_all(&output_root)?;
    if staged_package_root.exists() {
        fs::remove_dir_all(&staged_package_root).ok();
    }
    fs::create_dir_all(&staged_package_root)?;

    let lib_paths = build_targets(
        &metadata,
        package,
        &lib_name,
        &target_triples,
        options.build_mode,
        options.lib_type,
    )?;

    let lib_paths = coalesce_libraries(&target_triples, &lib_paths, &output_root)?;

    generate_swift_bindings(&lib_paths[0], &generated_dir, &metadata)?;

    let xcframework_path = output_root.join(format!("{xcframework_name}.xcframework"));
    let staged_xcframework_path =
        output_root.join(format!("._{xcframework_name}.next.xcframework"));
    if staged_xcframework_path.exists() {
        fs::remove_dir_all(&staged_xcframework_path).ok();
    }
    create_xcframework(&lib_paths, &generated_dir, &staged_xcframework_path)?;
    patch_xcframework(&staged_xcframework_path, &generated_dir, &xcframework_name)?;
    remove_xcframeworks_except(&output_root, Some(&staged_xcframework_path))?;
    fs::rename(&staged_xcframework_path, &xcframework_path)?;

    let staged_package_xcframework =
        staged_package_root.join(format!("{xcframework_name}.xcframework"));
    copy_dir(&xcframework_path, &staged_package_xcframework)?;

    let sources_dir = staged_package_root.join("Sources").join(&package_name);
    fs::create_dir_all(&sources_dir)?;
    copy_generated_sources(&generated_dir, &sources_dir)?;
    if options.swift_bridges {
        let vm_metas = load_vm_metadata(&options.crate_path, &options.metadata_bin)?;
        generate_swift_bridges(&sources_dir, &vm_metas, options.bindings.as_ref())?;
    }

    remove_stale_manifests(&staged_package_root)?;
    match options.format {
        PackageFormat::Spm => {
            write_spm_manifest(&staged_package_root, &package_name, &xcframework_name)?;
        }
        PackageFormat::Pod => {
            write_podspec(&staged_package_root, &package_name, &xcframework_name)?;
        }
    }

    replace_dir(&staged_package_root, &package_root)?;

    Ok(IosPackageReport {
        package_root,
        xcframework_path,
        package_name,
        lib_name,
        targets: target_triples,
    })
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
    if values.is_empty() {
        bail!("at least one iOS target must be configured");
    }
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
        platform_for_target(triple)?;
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
        let binding_files = generated_binding_files(out_dir.as_std_path(), crate_name)?;

        fs::copy(
            &binding_files.swift_source,
            sources.join(&binding_files.swift_file_name).as_std_path(),
        )?;
        fs::copy(
            &binding_files.ffi_header,
            headers
                .join(&binding_files.ffi_header_file_name)
                .as_std_path(),
        )?;

        let mut modulemap_part = fs::OpenOptions::new()
            .read(true)
            .open(&binding_files.ffi_modulemap)?;
        std::io::copy(&mut modulemap_part, &mut modulemap)?;
    }

    Ok(())
}

struct GeneratedBindingFiles {
    swift_source: PathBuf,
    swift_file_name: String,
    ffi_header: PathBuf,
    ffi_header_file_name: String,
    ffi_modulemap: PathBuf,
}

fn generated_binding_files(out_dir: &Path, crate_name: &str) -> Result<GeneratedBindingFiles> {
    let ffi_name = format!("{crate_name}FFI");
    let exact_swift = out_dir.join(format!("{crate_name}.swift"));
    let exact_header = out_dir.join(format!("{ffi_name}.h"));
    let exact_modulemap = out_dir.join(format!("{ffi_name}.modulemap"));

    let swift_source = if exact_swift.exists() {
        exact_swift
    } else {
        single_generated_file(out_dir, "Swift source", |path| {
            path.extension() == Some(OsStr::new("swift"))
        })?
    };
    let ffi_header = if exact_header.exists() {
        exact_header
    } else {
        single_generated_file(out_dir, "FFI header", |path| {
            path.extension() == Some(OsStr::new("h"))
                && path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .is_some_and(|stem| stem.ends_with("FFI"))
        })?
    };
    let ffi_modulemap = if exact_modulemap.exists() {
        exact_modulemap
    } else {
        single_generated_file(out_dir, "FFI modulemap", |path| {
            path.extension() == Some(OsStr::new("modulemap"))
                && path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .is_some_and(|stem| stem.ends_with("FFI"))
        })?
    };

    Ok(GeneratedBindingFiles {
        swift_file_name: file_name_string(&swift_source)?,
        ffi_header_file_name: file_name_string(&ffi_header)?,
        swift_source,
        ffi_header,
        ffi_modulemap,
    })
}

fn single_generated_file(
    out_dir: &Path,
    label: &str,
    predicate: impl Fn(&Path) -> bool,
) -> Result<PathBuf> {
    let matches = fs::read_dir(out_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && predicate(path))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => bail!(
            "{label} generated by UniFFI was not found in {}",
            out_dir.display()
        ),
        _ => bail!(
            "{label} generated by UniFFI is ambiguous in {}",
            out_dir.display()
        ),
    }
}

fn file_name_string(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("invalid generated binding filename {}", path.display()))
}

fn create_xcframework(lib_paths: &[PathBuf], generated_dir: &Path, output: &Path) -> Result<()> {
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

fn remove_stale_manifests(root: &Path) -> Result<()> {
    let spm_manifest = root.join("Package.swift");
    if spm_manifest.exists() {
        fs::remove_file(spm_manifest)?;
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("podspec")) {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn remove_xcframeworks_except(root: &Path, keep: Option<&Path>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("xcframework")) {
            if keep.is_some_and(|keep| keep == path) {
                continue;
            }
            fs::remove_dir_all(path)?;
        }
    }

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

fn replace_dir(staged: &Path, final_path: &Path) -> Result<()> {
    let backup = final_path.with_extension("prev");
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }

    if final_path.exists() {
        fs::rename(final_path, &backup)?;
    }

    if let Err(err) = fs::rename(staged, final_path) {
        if backup.exists() {
            let _ = fs::rename(&backup, final_path);
        }
        return Err(err).with_context(|| {
            format!(
                "failed to replace package directory {}",
                final_path.display()
            )
        });
    }

    if backup.exists() {
        fs::remove_dir_all(backup)?;
    }

    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct VmMeta {
    swift_bridge: String,
    mode: String,
    vm_type: String,
    observer: String,
    observer_method: String,
    state_type: String,
    diff_type: String,
    list_item_type: String,
    methods: Vec<VmMethod>,
    #[serde(default)]
    ir: Option<VmMetadata>,
    swift_code: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct VmMethod {
    name: String,
    args: Vec<VmArg>,
    ret: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct VmArg {
    name: String,
    ty: String,
}

fn load_vm_metadata(crate_path: &Path, metadata_bin: &str) -> Result<Vec<VmMeta>> {
    let manifest_path = crate_path.join("Cargo.toml");
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    cmd.args(["--manifest-path", manifest_path.to_str().unwrap()]);
    cmd.args(["--bin", metadata_bin]);
    cmd.arg("--quiet");
    let output = cmd
        .output()
        .with_context(|| format!("failed to run {metadata_bin}"))?;
    if !output.status.success() {
        bail!("{metadata_bin} failed");
    }
    let stdout = String::from_utf8(output.stdout)?;
    let metas: Vec<VmMeta> = serde_json::from_str(stdout.trim())
        .map_err(|err| anyhow!("failed to parse {metadata_bin} output: {err}"))?;
    Ok(metas)
}

fn generate_swift_bridges(
    sources_dir: &Path,
    metas: &[VmMeta],
    bindings: Option<&BindingsConfig>,
) -> Result<()> {
    let bridges_dir = sources_dir.join("Bridges");
    fs::create_dir_all(&bridges_dir)?;
    let irs = metas
        .iter()
        .filter_map(|meta| meta.ir.clone())
        .collect::<Vec<_>>();

    for meta in metas {
        if let Some(ir) = &meta.ir {
            let files = cross_kit_codegen::generate_swift_bridge(ir).map_err(|err| {
                anyhow!(
                    "failed to generate Swift bridge for {}: {err}",
                    meta.vm_type
                )
            })?;
            for file in files.files {
                fs::write(bridges_dir.join(file.path), file.contents)?;
            }
        } else {
            let content = meta
                .swift_code
                .as_deref()
                .filter(|code| !code.trim().is_empty())
                .ok_or_else(|| anyhow!("swift_code missing for {}", meta.vm_type))?;
            let filename = format!("{}.swift", meta.swift_bridge);
            fs::write(bridges_dir.join(filename), content)?;
        }
    }
    if let Some(bindings) = bindings {
        let files = cross_kit_codegen::generate_swift_root_container(&irs, bindings)
            .map_err(|err| anyhow!("failed to generate Swift root container: {err}"))?;
        for file in files.files {
            fs::write(bridges_dir.join(file.path), file.contents)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_step2_metadata_envelope_with_ir_and_swift_code() {
        let metas: Vec<VmMeta> = serde_json::from_str(
            r#"
            [
              {
                "schema_version": 1,
                "swift_bridge": "CounterViewModelBridge",
                "mode": "state",
                "vm_type": "CounterViewModel",
                "observer": "CounterObserver",
                "observer_method": "on_state",
                "state_type": "CounterState",
                "diff_type": "",
                "list_item_type": "",
                "factory_type": "AppViewModel",
                "factory_method": "make_counter_vm",
                "factory_bridge": "AppViewModelBridge",
                "methods": [
                  {
                    "name": "subscribe",
                    "args": [{"name": "observer", "ty": "Arc<dyn CounterObserver>"}],
                    "ret": "i64"
                  },
                  {
                    "name": "get_state",
                    "args": [],
                    "ret": "CounterState"
                  }
                ],
                "ir": {
                  "schema_version": 1,
                  "rust_type": "CounterViewModel",
                  "bridge_name": "CounterViewModelBridge",
                  "mode": "state",
                  "observer": {
                    "rust_type": "CounterObserver",
                    "method": "on_state"
                  },
                  "state_type": "CounterState",
                  "factory": {
                    "rust_type": "AppViewModel",
                    "method": "make_counter_vm",
                    "bridge_name": "AppViewModelBridge"
                  },
                  "methods": []
                },
                "swift_code": "public final class CounterViewModelBridge {}"
              }
            ]
            "#,
        )
        .unwrap();

        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].swift_bridge, "CounterViewModelBridge");
        assert!(metas[0].ir.is_some());
        assert_eq!(metas[0].methods[0].args[0].ty, "Arc<dyn CounterObserver>");
        assert_eq!(
            metas[0].swift_code.as_deref(),
            Some("public final class CounterViewModelBridge {}")
        );
    }

    #[test]
    fn writes_swift_bridge_files_from_ir_codegen() {
        let temp_dir = unique_temp_dir("ck-swift-packager-ir-bridges");
        let meta = VmMeta {
            swift_bridge: "CounterViewModelBridge".to_string(),
            mode: "state".to_string(),
            vm_type: "CounterViewModel".to_string(),
            observer: "CounterObserver".to_string(),
            observer_method: "on_state".to_string(),
            state_type: "CounterState".to_string(),
            diff_type: String::new(),
            list_item_type: String::new(),
            methods: Vec::new(),
            ir: Some(counter_ir()),
            swift_code: None,
        };

        generate_swift_bridges(&temp_dir, &[meta], None).unwrap();

        let bridge_file = temp_dir
            .join("Bridges")
            .join("CounterViewModelBridge.swift");
        let content = fs::read_to_string(&bridge_file).unwrap();
        assert!(content.contains("// Generated by cross-kit-codegen."));
        assert!(content.contains("public final class CounterViewModelBridge"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn writes_swift_root_container_when_bindings_are_configured() {
        let temp_dir = unique_temp_dir("ck-swift-packager-root-container");
        let metas = vec![vm_meta(app_ir()), vm_meta(counter_ir())];
        let bindings = BindingsConfig {
            root_vm: "AppViewModel".to_string(),
            container_name: "CrossKitSharedBridge".to_string(),
        };

        generate_swift_bridges(&temp_dir, &metas, Some(&bindings)).unwrap();

        let root_file = temp_dir.join("Bridges").join("CrossKitSharedBridge.swift");
        let counter_file = temp_dir
            .join("Bridges")
            .join("CounterViewModelBridge.swift");
        let root_code = fs::read_to_string(root_file).unwrap();
        assert!(counter_file.exists());
        assert!(root_code.contains("public final class CrossKitSharedBridge"));
        assert!(root_code.contains("public let app: AppViewModelBridge"));
        assert!(root_code.contains("public let counter: CounterViewModelBridge"));
        assert!(root_code.contains("counter.objectWillChange.sink"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn writes_swift_bridge_files_from_legacy_compatibility_field() {
        let temp_dir = unique_temp_dir("ck-swift-packager-bridges");
        let meta = VmMeta {
            swift_bridge: "CounterViewModelBridge".to_string(),
            mode: "state".to_string(),
            vm_type: "CounterViewModel".to_string(),
            observer: "CounterObserver".to_string(),
            observer_method: "on_state".to_string(),
            state_type: "CounterState".to_string(),
            diff_type: String::new(),
            list_item_type: String::new(),
            methods: Vec::new(),
            ir: None,
            swift_code: Some("public final class CounterViewModelBridge {}".to_string()),
        };

        generate_swift_bridges(&temp_dir, &[meta], None).unwrap();

        let bridge_file = temp_dir
            .join("Bridges")
            .join("CounterViewModelBridge.swift");
        assert_eq!(
            fs::read_to_string(&bridge_file).unwrap(),
            "public final class CounterViewModelBridge {}"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn rejects_metadata_without_swift_code_during_compatibility_window() {
        let temp_dir = unique_temp_dir("ck-swift-packager-missing-swift-code");
        let meta = VmMeta {
            swift_bridge: "CounterViewModelBridge".to_string(),
            mode: "state".to_string(),
            vm_type: "CounterViewModel".to_string(),
            observer: "CounterObserver".to_string(),
            observer_method: "on_state".to_string(),
            state_type: "CounterState".to_string(),
            diff_type: String::new(),
            list_item_type: String::new(),
            methods: Vec::new(),
            ir: None,
            swift_code: None,
        };

        let err = generate_swift_bridges(&temp_dir, &[meta], None).unwrap_err();
        assert!(err.to_string().contains("swift_code missing"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn default_options_match_compatibility_command_defaults() {
        let options = IosPackageOptions::default();

        assert_eq!(options.crate_path, PathBuf::from("."));
        assert_eq!(options.build_mode, BuildMode::Release);
        assert_eq!(options.lib_type, LibType::Static);
        assert_eq!(options.format, PackageFormat::Spm);
        assert_eq!(options.metadata_bin, "ck_vm_metadata");
        assert!(!options.swift_bridges);
    }

    #[test]
    fn parses_package_option_enums_with_actionable_errors() {
        assert_eq!("debug".parse::<BuildMode>().unwrap(), BuildMode::Debug);
        assert_eq!("release".parse::<BuildMode>().unwrap(), BuildMode::Release);
        assert_eq!("static".parse::<LibType>().unwrap(), LibType::Static);
        assert_eq!("dynamic".parse::<LibType>().unwrap(), LibType::Dynamic);
        assert_eq!("spm".parse::<PackageFormat>().unwrap(), PackageFormat::Spm);
        assert_eq!("pod".parse::<PackageFormat>().unwrap(), PackageFormat::Pod);

        assert!(
            "fast"
                .parse::<BuildMode>()
                .unwrap_err()
                .to_string()
                .contains("unsupported build mode")
        );
        assert!(
            "shared"
                .parse::<LibType>()
                .unwrap_err()
                .to_string()
                .contains("unsupported iOS lib type")
        );
        assert!(
            "zip"
                .parse::<PackageFormat>()
                .unwrap_err()
                .to_string()
                .contains("unsupported iOS package format")
        );
    }

    #[test]
    fn resolves_supported_apple_target_aliases() {
        let targets = resolve_targets(Some(vec![
            "ios".to_string(),
            "ios-sim".to_string(),
            "ios-sim-x86_64".to_string(),
            "macos".to_string(),
            "macos-x86_64".to_string(),
        ]))
        .unwrap();

        assert_eq!(
            targets,
            [
                "aarch64-apple-ios",
                "aarch64-apple-ios-sim",
                "x86_64-apple-ios",
                "aarch64-apple-darwin",
                "x86_64-apple-darwin"
            ]
            .map(str::to_string)
        );
    }

    #[test]
    fn rejects_empty_and_unsupported_target_sets_before_building() {
        let empty = resolve_targets(Some(Vec::new())).unwrap_err();
        assert!(
            empty
                .to_string()
                .contains("at least one iOS target must be configured")
        );

        let unsupported =
            resolve_targets(Some(vec!["aarch64-apple-visionos".to_string()])).unwrap_err();
        assert!(
            unsupported
                .to_string()
                .contains("unsupported apple target: aarch64-apple-visionos")
        );
    }

    #[test]
    fn computes_library_output_paths_for_profiles_and_library_types() {
        let target_dir = Utf8Path::new("/tmp/cross-kit-target");

        assert_eq!(
            lib_output_path(
                target_dir,
                "aarch64-apple-ios",
                BuildMode::Debug,
                "cross_kit_shared",
                LibType::Static
            )
            .unwrap(),
            PathBuf::from("/tmp/cross-kit-target/aarch64-apple-ios/debug/libcross_kit_shared.a")
        );
        assert_eq!(
            lib_output_path(
                target_dir,
                "aarch64-apple-ios-sim",
                BuildMode::Release,
                "cross_kit_shared",
                LibType::Dynamic
            )
            .unwrap(),
            PathBuf::from(
                "/tmp/cross-kit-target/aarch64-apple-ios-sim/release/libcross_kit_shared.dylib"
            )
        );
    }

    #[test]
    fn writes_spm_manifest_and_podspec() {
        let temp_dir = unique_temp_dir("ck-swift-packager-manifests");
        fs::create_dir_all(&temp_dir).unwrap();

        write_spm_manifest(&temp_dir, "CrossKitShared", "cross_kit_sharedFFI").unwrap();
        let spm = fs::read_to_string(temp_dir.join("Package.swift")).unwrap();
        assert!(spm.contains("name: \"CrossKitShared\""));
        assert!(spm.contains("path: \"./cross_kit_sharedFFI.xcframework\""));

        write_podspec(&temp_dir, "CrossKitShared", "cross_kit_sharedFFI").unwrap();
        let podspec = fs::read_to_string(temp_dir.join("CrossKitShared.podspec")).unwrap();
        assert!(podspec.contains("s.name = \"CrossKitShared\""));
        assert!(podspec.contains("cross_kit_sharedFFI.xcframework"));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn removes_stale_manifests_before_writing_selected_format() {
        let temp_dir = unique_temp_dir("ck-swift-packager-stale-manifests");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("Package.swift"), "stale spm").unwrap();
        fs::write(temp_dir.join("OldPackage.podspec"), "stale pod").unwrap();
        fs::write(temp_dir.join("notes.txt"), "keep").unwrap();

        remove_stale_manifests(&temp_dir).unwrap();

        assert!(!temp_dir.join("Package.swift").exists());
        assert!(!temp_dir.join("OldPackage.podspec").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.join("notes.txt")).unwrap(),
            "keep"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn removes_stale_xcframeworks_before_copying_selected_binary() {
        let temp_dir = unique_temp_dir("ck-swift-packager-stale-xcframeworks");
        fs::create_dir_all(temp_dir.join("OldFFI.xcframework")).unwrap();
        fs::create_dir_all(temp_dir.join("CurrentFFI.xcframework")).unwrap();
        fs::write(temp_dir.join("notes.txt"), "keep").unwrap();

        remove_xcframeworks_except(&temp_dir, None).unwrap();

        assert!(!temp_dir.join("OldFFI.xcframework").exists());
        assert!(!temp_dir.join("CurrentFFI.xcframework").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.join("notes.txt")).unwrap(),
            "keep"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preserves_staged_xcframework_during_replacement_cleanup() {
        let temp_dir = unique_temp_dir("ck-swift-packager-keep-staged-xcframework");
        let staged = temp_dir.join("._CurrentFFI.next.xcframework");
        fs::create_dir_all(temp_dir.join("OldFFI.xcframework")).unwrap();
        fs::create_dir_all(&staged).unwrap();

        remove_xcframeworks_except(&temp_dir, Some(&staged)).unwrap();

        assert!(!temp_dir.join("OldFFI.xcframework").exists());
        assert!(staged.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn copies_swift_sources_and_patches_xcframework_headers() {
        let temp_dir = unique_temp_dir("ck-swift-packager-copy-patch");
        let generated = temp_dir.join("generated");
        let sources = generated.join("sources");
        let headers = generated.join("headers");
        let dest = temp_dir.join("dest");
        let xcframework = temp_dir.join("CrossKitSharedFFI.xcframework");
        let ios_slice = xcframework.join("ios-arm64");
        let sim_slice = xcframework.join("ios-arm64_x86_64-simulator");

        fs::create_dir_all(&sources).unwrap();
        fs::create_dir_all(&headers).unwrap();
        fs::create_dir_all(ios_slice.join("Headers")).unwrap();
        fs::create_dir_all(sim_slice.join("Headers")).unwrap();
        fs::write(sources.join("cross_kit_shared.swift"), "swift").unwrap();
        fs::write(sources.join("notes.txt"), "ignore").unwrap();
        fs::write(headers.join("cross_kit_sharedFFI.h"), "header").unwrap();
        fs::write(headers.join("module.modulemap"), "module").unwrap();
        fs::create_dir_all(&dest).unwrap();

        copy_generated_sources(&generated, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("cross_kit_shared.swift")).unwrap(),
            "swift"
        );
        assert!(!dest.join("notes.txt").exists());

        patch_xcframework(&xcframework, &generated, "cross_kit_sharedFFI").unwrap();
        assert!(
            ios_slice
                .join("Headers")
                .join("cross_kit_sharedFFI")
                .join("cross_kit_sharedFFI.h")
                .exists()
        );
        assert!(
            sim_slice
                .join("Headers")
                .join("cross_kit_sharedFFI")
                .join("module.modulemap")
                .exists()
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn resolves_uniffi_outputs_named_after_crate() {
        let temp_dir = unique_temp_dir("ck-swift-packager-exact-uniffi-names");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("cross_kit_shared.swift"), "swift").unwrap();
        fs::write(temp_dir.join("cross_kit_sharedFFI.h"), "header").unwrap();
        fs::write(temp_dir.join("cross_kit_sharedFFI.modulemap"), "module").unwrap();

        let files = generated_binding_files(&temp_dir, "cross_kit_shared").unwrap();

        assert_eq!(files.swift_file_name, "cross_kit_shared.swift");
        assert_eq!(files.ffi_header_file_name, "cross_kit_sharedFFI.h");
        assert_eq!(files.swift_source, temp_dir.join("cross_kit_shared.swift"));
        assert_eq!(files.ffi_header, temp_dir.join("cross_kit_sharedFFI.h"));
        assert_eq!(
            files.ffi_modulemap,
            temp_dir.join("cross_kit_sharedFFI.modulemap")
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn resolves_uniffi_outputs_named_after_swift_module() {
        let temp_dir = unique_temp_dir("ck-swift-packager-module-uniffi-names");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("CrossKitMinimalCounterShared.swift"), "swift").unwrap();
        fs::write(temp_dir.join("CrossKitMinimalCounterSharedFFI.h"), "header").unwrap();
        fs::write(
            temp_dir.join("CrossKitMinimalCounterSharedFFI.modulemap"),
            "module",
        )
        .unwrap();

        let files = generated_binding_files(&temp_dir, "cross_kit_minimal_counter_shared").unwrap();

        assert_eq!(files.swift_file_name, "CrossKitMinimalCounterShared.swift");
        assert_eq!(
            files.ffi_header_file_name,
            "CrossKitMinimalCounterSharedFFI.h"
        );
        assert_eq!(
            files.swift_source,
            temp_dir.join("CrossKitMinimalCounterShared.swift")
        );
        assert_eq!(
            files.ffi_header,
            temp_dir.join("CrossKitMinimalCounterSharedFFI.h")
        );
        assert_eq!(
            files.ffi_modulemap,
            temp_dir.join("CrossKitMinimalCounterSharedFFI.modulemap")
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn coalesces_single_arch_libraries_without_lipo() {
        let libs = vec![
            PathBuf::from("/tmp/libcross_kit_shared_ios.a"),
            PathBuf::from("/tmp/libcross_kit_shared_sim.a"),
        ];
        let targets = vec![
            "aarch64-apple-ios".to_string(),
            "aarch64-apple-ios-sim".to_string(),
        ];
        let output_root = unique_temp_dir("ck-swift-packager-lipo-root");
        fs::create_dir_all(&output_root).unwrap();

        let coalesced = coalesce_libraries(&targets, &libs, &output_root).unwrap();

        assert_eq!(coalesced, libs);
        let _ = fs::remove_dir_all(output_root);
    }

    #[test]
    fn rejects_missing_manifest_before_running_build_tools() {
        let temp_dir = unique_temp_dir("ck-swift-packager-missing-manifest");
        fs::create_dir_all(&temp_dir).unwrap();

        let err = load_metadata(&temp_dir).unwrap_err();

        assert!(err.to_string().contains("Cargo.toml not found"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn metadata_loader_errors_use_configured_binary_name() {
        let temp_dir = unique_temp_dir("ck-swift-packager-custom-metadata-bin");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(
            temp_dir.join("Cargo.toml"),
            r#"
            [package]
            name = "metadata-bin-test"
            version = "0.1.0"
            edition = "2024"
            "#,
        )
        .unwrap();

        let err = load_vm_metadata(&temp_dir, "custom_metadata").unwrap_err();

        assert!(err.to_string().contains("custom_metadata failed"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn replaces_package_directory_after_staged_contents_are_ready() {
        let temp_dir = unique_temp_dir("ck-swift-packager-replace-dir");
        let final_dir = temp_dir.join("CrossKitShared");
        let staged_dir = temp_dir.join("._CrossKitShared.next");
        fs::create_dir_all(&final_dir).unwrap();
        fs::create_dir_all(&staged_dir).unwrap();
        fs::write(final_dir.join("old.txt"), "old").unwrap();
        fs::write(staged_dir.join("new.txt"), "new").unwrap();

        replace_dir(&staged_dir, &final_dir).unwrap();

        assert!(!staged_dir.exists());
        assert!(!temp_dir.join("CrossKitShared.prev").exists());
        assert!(!final_dir.join("old.txt").exists());
        assert_eq!(
            fs::read_to_string(final_dir.join("new.txt")).unwrap(),
            "new"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn counter_ir() -> VmMetadata {
        serde_json::from_value(serde_json::json!({
            "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
            "rust_type": "CounterViewModel",
            "bridge_name": "CounterViewModelBridge",
            "mode": "state",
            "observer": {
                "rust_type": "CounterObserver",
                "method": "on_state"
            },
            "state_type": "CounterState",
            "factory": {
                "rust_type": "AppViewModel",
                "method": "make_counter_vm",
                "bridge_name": "AppViewModelBridge"
            },
            "methods": [
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn CounterObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "unsubscribe",
                    "args": [{"name": "id", "rust_type": "i64"}],
                    "return_type": "unit"
                },
                {
                    "name": "get_state",
                    "args": [],
                    "return_type": "CounterState"
                }
            ]
        }))
        .unwrap()
    }

    fn app_ir() -> VmMetadata {
        serde_json::from_value(serde_json::json!({
            "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
            "rust_type": "AppViewModel",
            "bridge_name": "AppViewModelBridge",
            "mode": "state",
            "observer": {
                "rust_type": "AppObserver",
                "method": "on_state"
            },
            "state_type": "AppState",
            "methods": [
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn AppObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "get_state",
                    "args": [],
                    "return_type": "AppState"
                },
                {
                    "name": "new",
                    "args": [{"name": "initial", "rust_type": "i32"}],
                    "return_type": "Arc<Self>"
                },
                {
                    "name": "make_counter_vm",
                    "args": [],
                    "return_type": "Arc<CounterViewModel>"
                }
            ]
        }))
        .unwrap()
    }

    fn vm_meta(ir: VmMetadata) -> VmMeta {
        VmMeta {
            swift_bridge: ir.bridge_name.clone(),
            mode: "state".to_string(),
            vm_type: ir.rust_type.clone(),
            observer: ir
                .observer
                .as_ref()
                .map(|observer| observer.rust_type.clone())
                .unwrap_or_default(),
            observer_method: ir
                .observer
                .as_ref()
                .map(|observer| observer.method.clone())
                .unwrap_or_default(),
            state_type: ir.state_type.clone().unwrap_or_default(),
            diff_type: ir.diff_type.clone().unwrap_or_default(),
            list_item_type: ir.list_item_type.clone().unwrap_or_default(),
            methods: Vec::new(),
            ir: Some(ir),
            swift_code: None,
        }
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
