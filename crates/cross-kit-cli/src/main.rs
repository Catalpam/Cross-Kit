use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use cross_kit_core::{CONFIG_FILE_NAME, CrossKitConfig, VmMetadata};
use cross_kit_packager_android::AndroidPackageOptions;
use cross_kit_packager_ios::{BuildMode, IosPackageOptions, LibType, PackageFormat};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(author, version, about = "Cross-Kit project tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build platform packages from a Cross-Kit project.
    Ios {
        #[command(subcommand)]
        command: IosCommand,
    },
    /// Generate platform bridge sources.
    Gen {
        #[command(subcommand)]
        command: GenCommand,
    },
    /// Android build helpers before AAR packaging exists.
    Android {
        #[command(subcommand)]
        command: AndroidCommand,
    },
}

#[derive(Debug, Subcommand)]
enum IosCommand {
    /// Generate an iOS Swift Package and XCFramework.
    Package {
        /// Path to cross-kit.toml.
        #[arg(long, default_value = CONFIG_FILE_NAME)]
        config: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum GenCommand {
    /// Generate bridge sources for a platform.
    Bridges {
        /// Platform to generate.
        #[arg(long)]
        platform: Platform,
        /// Path to cross-kit.toml.
        #[arg(long, default_value = CONFIG_FILE_NAME)]
        config: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum AndroidCommand {
    /// Build Android native libraries and UniFFI Kotlin bindings.
    BuildNative {
        /// Path to cross-kit.toml.
        #[arg(long, default_value = CONFIG_FILE_NAME)]
        config: PathBuf,
    },
    /// Generate and build an Android AAR.
    Package {
        /// Path to cross-kit.toml.
        #[arg(long, default_value = CONFIG_FILE_NAME)]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Platform {
    Android,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Ios {
            command: IosCommand::Package { config },
        } => {
            let options = load_ios_options(&config)?;
            let report = cross_kit_packager_ios::package_ios(&options)?;
            println!("iOS package written to {}", report.package_root.display());
        }
        Command::Gen {
            command:
                GenCommand::Bridges {
                    platform: Platform::Android,
                    config,
                },
        } => {
            let report = generate_android_bridges(&config)?;
            println!(
                "Android bridges written to {}",
                report.bridge_output.display()
            );
        }
        Command::Android {
            command: AndroidCommand::BuildNative { config },
        } => {
            let report = build_android_native(&config)?;
            println!(
                "Android native libraries written to {}; Kotlin bindings written to {}",
                report.jni_libs_output.display(),
                report.binding_output.display()
            );
        }
        Command::Android {
            command: AndroidCommand::Package { config },
        } => {
            let options = load_android_package_options(&config)?;
            let report = cross_kit_packager_android::package_android(&options)?;
            println!(
                "Android AAR written to {}; local Maven repo written to {}",
                report.aar.display(),
                report.maven_repo.display()
            );
        }
    }
    Ok(())
}

fn load_ios_options(config_path: &Path) -> Result<IosPackageOptions> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let config = CrossKitConfig::from_toml_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    ios_options_from_config(config_path, &config)
}

fn ios_options_from_config(
    config_path: &Path,
    config: &CrossKitConfig,
) -> Result<IosPackageOptions> {
    let ios = config
        .ios
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [ios] section in {}", config_path.display()))?;
    if ios.package_name.trim().is_empty() {
        bail!("[ios].package_name must not be empty");
    }
    if config.shared.crate_path.trim().is_empty() {
        bail!("[shared].crate_path must not be empty");
    }

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(IosPackageOptions {
        crate_path: resolve_relative(config_dir, &config.shared.crate_path),
        package_name: Some(ios.package_name.clone()),
        package: config.shared.package.clone(),
        lib_name: config.shared.lib_name.clone(),
        output: ios
            .output
            .as_ref()
            .map(|output| resolve_relative(config_dir, output)),
        xcframework_name: ios.xcframework_name.clone(),
        targets: Some(ios.targets.clone()),
        build_mode: BuildMode::from_str(&ios.build_mode)?,
        lib_type: LibType::from_str(&ios.lib_type)?,
        format: PackageFormat::from_str(&ios.format)?,
        swift_bridges: ios.swift_bridges,
        metadata_bin: config.shared.metadata_bin.clone(),
        bindings: config.bindings.clone(),
    })
}

fn resolve_relative(base: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

#[derive(Debug)]
struct AndroidPaths {
    crate_path: PathBuf,
    package_name: String,
    bridge_output: PathBuf,
    binding_output: PathBuf,
    jni_libs_output: PathBuf,
    targets: Vec<String>,
    build_mode: String,
    lib_name: String,
    metadata_bin: String,
}

#[derive(Debug)]
struct AndroidBridgeReport {
    bridge_output: PathBuf,
}

#[derive(Debug)]
struct AndroidNativeReport {
    binding_output: PathBuf,
    jni_libs_output: PathBuf,
}

#[derive(Debug)]
struct AndroidNativePlan {
    manifest: PathBuf,
    library: PathBuf,
}

fn android_paths_from_config(config_path: &Path, config: &CrossKitConfig) -> Result<AndroidPaths> {
    let android = config
        .android
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [android] section in {}", config_path.display()))?;
    if config.shared.crate_path.trim().is_empty() {
        bail!("[shared].crate_path must not be empty");
    }
    if android.build_mode != "debug" && android.build_mode != "release" {
        bail!(
            "unsupported Android build mode '{}'; expected 'debug' or 'release'",
            android.build_mode
        );
    }
    if android.targets.is_empty()
        || android
            .targets
            .iter()
            .any(|target| target.trim().is_empty())
    {
        bail!("[android].targets must contain at least one non-empty target");
    }

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let generated_root = android
        .output
        .as_ref()
        .map(|output| resolve_relative(config_dir, output))
        .unwrap_or_else(|| resolve_relative(config_dir, "android/app/build/generated/cross-kit"));
    Ok(AndroidPaths {
        crate_path: resolve_relative(config_dir, &config.shared.crate_path),
        package_name: android
            .package_name
            .clone()
            .unwrap_or_else(|| "com.crosskit.shared".to_string()),
        bridge_output: generated_root.join("bridges"),
        binding_output: generated_root.join("uniffi"),
        jni_libs_output: android
            .jni_libs_output
            .as_ref()
            .map(|output| resolve_relative(config_dir, output))
            .unwrap_or_else(|| resolve_relative(config_dir, "android/app/src/main/jniLibs")),
        targets: android.targets.clone(),
        build_mode: android.build_mode.clone(),
        lib_name: config
            .shared
            .lib_name
            .clone()
            .unwrap_or_else(|| "cross_kit_shared".to_string()),
        metadata_bin: config.shared.metadata_bin.clone(),
    })
}

fn load_android_package_options(config_path: &Path) -> Result<AndroidPackageOptions> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let config = CrossKitConfig::from_toml_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    android_package_options_from_config(config_path, &config)
}

fn android_package_options_from_config(
    config_path: &Path,
    config: &CrossKitConfig,
) -> Result<AndroidPackageOptions> {
    let android = config
        .android
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [android] section in {}", config_path.display()))?;
    if config.shared.crate_path.trim().is_empty() {
        bail!("[shared].crate_path must not be empty");
    }
    if android.build_mode != "debug" && android.build_mode != "release" {
        bail!(
            "unsupported Android build mode '{}'; expected 'debug' or 'release'",
            android.build_mode
        );
    }
    if android.targets.is_empty()
        || android
            .targets
            .iter()
            .any(|target| target.trim().is_empty())
    {
        bail!("[android].targets must contain at least one non-empty target");
    }

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let output = android
        .package_output
        .as_ref()
        .map(|output| resolve_relative(config_dir, output))
        .unwrap_or_else(|| resolve_relative(config_dir, "dist/android"));
    let gradle_project = android
        .gradle_project_output
        .as_ref()
        .map(|output| resolve_relative(config_dir, output))
        .unwrap_or_else(|| output.join("gradle-project"));
    let module_name = android
        .module_name
        .clone()
        .unwrap_or_else(|| "crosskitshared".to_string());
    let mut maven = android.maven.clone();
    if !maven.artifact_id_explicit {
        maven.artifact_id = module_name.clone();
    }
    Ok(AndroidPackageOptions {
        crate_path: resolve_relative(config_dir, &config.shared.crate_path),
        package_name: android
            .package_name
            .clone()
            .unwrap_or_else(|| "com.crosskit.shared".to_string()),
        lib_name: config
            .shared
            .lib_name
            .clone()
            .unwrap_or_else(|| "cross_kit_shared".to_string()),
        output,
        gradle_project,
        module_name,
        gradle_executable: android
            .gradle_executable
            .as_ref()
            .map(|path| resolve_relative(config_dir, path))
            .unwrap_or_else(|| resolve_relative(config_dir, "android/gradlew")),
        java_home: android
            .java_home
            .as_ref()
            .map(|path| resolve_relative(config_dir, path)),
        targets: android.targets.clone(),
        build_mode: android.build_mode.clone(),
        metadata_bin: config.shared.metadata_bin.clone(),
        maven,
        bindings: config.bindings.clone(),
    })
}

fn generate_android_bridges(config_path: &Path) -> Result<AndroidBridgeReport> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let config = CrossKitConfig::from_toml_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let paths = android_paths_from_config(config_path, &config)?;
    let metadatas = load_vm_metadatas(&paths)?;

    replace_generated_dir(&paths.bridge_output)?;
    for metadata in &metadatas {
        let files = cross_kit_codegen::generate_kotlin_bridge(metadata, &paths.package_name)?;
        for file in files.files {
            let path = paths.bridge_output.join(file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, file.contents)?;
        }
    }
    if let Some(bindings) = &config.bindings {
        let files = cross_kit_codegen::generate_kotlin_root_container(
            &metadatas,
            bindings,
            &paths.package_name,
        )?;
        for file in files.files {
            let path = paths.bridge_output.join(file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, file.contents)?;
        }
    }

    Ok(AndroidBridgeReport {
        bridge_output: paths.bridge_output,
    })
}

fn build_android_native(config_path: &Path) -> Result<AndroidNativeReport> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config {}", config_path.display()))?;
    let config = CrossKitConfig::from_toml_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let paths = android_paths_from_config(config_path, &config)?;
    let plan = android_native_plan(&paths)?;

    fs::create_dir_all(&paths.jni_libs_output)?;
    let command = cargo_ndk_command(&paths, &plan.manifest);
    run_status(command, "cargo ndk build")?;
    generate_uniffi_kotlin_bindings(&paths, &plan.manifest, &plan.library)?;

    Ok(AndroidNativeReport {
        binding_output: paths.binding_output,
        jni_libs_output: paths.jni_libs_output,
    })
}

fn android_native_plan(paths: &AndroidPaths) -> Result<AndroidNativePlan> {
    let first_target = paths
        .targets
        .first()
        .ok_or_else(|| anyhow::anyhow!("[android].targets must not be empty"))?;
    Ok(AndroidNativePlan {
        manifest: paths.crate_path.join("Cargo.toml"),
        library: paths
            .jni_libs_output
            .join(first_target)
            .join(format!("lib{}.so", paths.lib_name)),
    })
}

fn cargo_ndk_command(paths: &AndroidPaths, manifest: &Path) -> ProcessCommand {
    let mut command = ProcessCommand::new("cargo");
    command.arg("ndk");
    for target in &paths.targets {
        command.arg("-t").arg(target);
    }
    command
        .arg("-o")
        .arg(&paths.jni_libs_output)
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest);
    if paths.build_mode == "release" {
        command.arg("--release");
    }
    command
}

fn load_vm_metadatas(paths: &AndroidPaths) -> Result<Vec<VmMetadata>> {
    let manifest = paths.crate_path.join("Cargo.toml");
    let output = ProcessCommand::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--bin")
        .arg(&paths.metadata_bin)
        .output()
        .with_context(|| format!("failed to run metadata binary {}", paths.metadata_bin))?;
    if !output.status.success() {
        bail!(
            "metadata binary {} failed:\n{}",
            paths.metadata_bin,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let values: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("failed to parse metadata JSON from {}", paths.metadata_bin))?;
    values
        .into_iter()
        .map(|value| {
            let ir = value.get("ir").cloned().unwrap_or(value);
            serde_json::from_value(ir).context("failed to parse VM metadata IR")
        })
        .collect()
}

fn generate_uniffi_kotlin_bindings(
    paths: &AndroidPaths,
    manifest: &Path,
    library: &Path,
) -> Result<()> {
    if !library.exists() {
        bail!("expected Android library at {}", library.display());
    }
    replace_generated_dir(&paths.binding_output)?;
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest)
        .exec()
        .context("failed to load Cargo metadata for UniFFI")?;
    let config_supplier = uniffi_bindgen::cargo_metadata::CrateConfigSupplier::from(metadata);
    let library = utf8_path(library)?;
    let out_dir = utf8_path(&paths.binding_output)?;
    uniffi_bindgen::library_mode::generate_bindings(
        &library,
        None,
        &uniffi_bindgen::bindings::KotlinBindingGenerator,
        &config_supplier,
        None,
        &out_dir,
        false,
    )
    .context("failed to generate UniFFI Kotlin bindings")?;
    Ok(())
}

fn replace_generated_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))
}

fn utf8_path(path: &Path) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|path| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn run_status(mut command: ProcessCommand, name: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to spawn {name}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("{name} failed with status {status}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cross_kit_core::AndroidMavenConfig;
    use std::{env, process};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("cross-kit-cli-test-{}-{name}", process::id()))
    }

    struct RemoveFileOnDrop(PathBuf);

    impl Drop for RemoveFileOnDrop {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn maps_ios_config_to_packager_options_with_relative_paths() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"
            package = "shared"
            lib_name = "cross_kit_shared"
            metadata_bin = "ck_vm_metadata"

            [bindings]
            root_vm = "AppViewModel"
            container_name = "CrossKitSharedBridge"

            [ios]
            package_name = "CrossKitShared"
            output = "dist/ios"
            targets = ["ios", "ios-sim-x86_64"]
            build_mode = "debug"
            lib_type = "dynamic"
            format = "pod"
            swift_bridges = false
            "#,
        )
        .unwrap();

        let options =
            ios_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap();

        assert_eq!(options.crate_path, PathBuf::from("/tmp/project/shared"));
        assert_eq!(options.package_name.as_deref(), Some("CrossKitShared"));
        assert_eq!(options.package.as_deref(), Some("shared"));
        assert_eq!(options.lib_name.as_deref(), Some("cross_kit_shared"));
        assert_eq!(options.output, Some(PathBuf::from("/tmp/project/dist/ios")));
        assert_eq!(
            options.targets,
            Some(vec!["ios".to_string(), "ios-sim-x86_64".to_string()])
        );
        assert_eq!(options.build_mode, BuildMode::Debug);
        assert_eq!(options.lib_type, LibType::Dynamic);
        assert_eq!(options.format, PackageFormat::Pod);
        assert!(!options.swift_bridges);
        let bindings = options.bindings.unwrap();
        assert_eq!(bindings.root_vm, "AppViewModel");
        assert_eq!(bindings.container_name, "CrossKitSharedBridge");
    }

    #[test]
    fn loads_counter_list_example_config_after_directory_migration() {
        let repo_root = repo_root();
        let config_path = repo_root.join("examples/counter-list/cross-kit.toml");

        let options = load_ios_options(&config_path).unwrap();
        let content = fs::read_to_string(&config_path).unwrap();
        let config = CrossKitConfig::from_toml_str(&content).unwrap();
        let android = android_paths_from_config(&config_path, &config).unwrap();
        let android_package = android_package_options_from_config(&config_path, &config).unwrap();

        assert_eq!(
            options.crate_path,
            repo_root.join("examples/counter-list/shared")
        );
        assert_eq!(
            options.output,
            Some(repo_root.join("examples/counter-list/dist/ios"))
        );
        assert_eq!(options.package_name.as_deref(), Some("CrossKitShared"));
        assert_eq!(options.package.as_deref(), Some("shared"));
        assert_eq!(options.lib_name.as_deref(), Some("cross_kit_shared"));
        assert_eq!(options.metadata_bin, "ck_vm_metadata");
        assert_eq!(
            options
                .bindings
                .as_ref()
                .map(|bindings| bindings.root_vm.as_str()),
            Some("AppViewModel")
        );
        assert_eq!(
            android.crate_path,
            repo_root.join("examples/counter-list/shared")
        );
        assert_eq!(android.package_name, "com.crosskit.shared");
        assert_eq!(
            android.bridge_output,
            repo_root.join("examples/counter-list/android/app/build/generated/cross-kit/bridges")
        );
        assert_eq!(
            android.binding_output,
            repo_root.join("examples/counter-list/android/app/build/generated/cross-kit/uniffi")
        );
        assert_eq!(
            android.jni_libs_output,
            repo_root.join("examples/counter-list/dist/android/jniLibs")
        );
        assert_eq!(
            android_package.output,
            repo_root.join("examples/counter-list/dist/android")
        );
        assert_eq!(
            android_package.gradle_project,
            repo_root.join("examples/counter-list/dist/android/gradle-project")
        );
        assert_eq!(android_package.module_name, "crosskitshared");
        assert_eq!(android_package.maven.group_id, "com.crosskit");
        assert_eq!(android_package.maven.artifact_id, "crosskitshared");
        assert_eq!(android_package.maven.version, "0.1.0");
        assert!(android_package.maven.artifact_id_explicit);
        assert_eq!(
            android_package.gradle_executable,
            repo_root.join("examples/counter-list/android/gradlew")
        );
        assert_eq!(android_package.java_home, None);
        assert_eq!(
            android_package
                .bindings
                .as_ref()
                .map(|bindings| bindings.container_name.as_str()),
            Some("CrossKitSharedBridge")
        );
    }

    #[test]
    fn loads_minimal_counter_example_config() {
        let repo_root = repo_root();
        let config_path = repo_root.join("examples/minimal-counter/cross-kit.toml");

        let options = load_ios_options(&config_path).unwrap();
        let content = fs::read_to_string(&config_path).unwrap();
        let config = CrossKitConfig::from_toml_str(&content).unwrap();
        let android = android_paths_from_config(&config_path, &config).unwrap();
        let android_package = android_package_options_from_config(&config_path, &config).unwrap();

        assert_eq!(
            options.crate_path,
            repo_root.join("examples/minimal-counter/shared")
        );
        assert_eq!(
            options.output,
            Some(repo_root.join("examples/minimal-counter/dist/ios"))
        );
        assert_eq!(
            options.package_name.as_deref(),
            Some("CrossKitMinimalCounterShared")
        );
        assert_eq!(options.package.as_deref(), Some("minimal-counter-shared"));
        assert_eq!(
            options.lib_name.as_deref(),
            Some("cross_kit_minimal_counter_shared")
        );
        assert_eq!(options.metadata_bin, "ck_minimal_counter_metadata");
        assert_eq!(
            options
                .bindings
                .as_ref()
                .map(|bindings| bindings.root_vm.as_str()),
            Some("CounterViewModel")
        );
        assert_eq!(
            options
                .bindings
                .as_ref()
                .map(|bindings| bindings.container_name.as_str()),
            Some("CrossKitMinimalCounterBridge")
        );
        assert_eq!(
            android.crate_path,
            repo_root.join("examples/minimal-counter/shared")
        );
        assert_eq!(android.package_name, "com.crosskit.minimalcounter.shared");
        assert_eq!(
            android.bridge_output,
            repo_root
                .join("examples/minimal-counter/android/app/build/generated/cross-kit/bridges")
        );
        assert_eq!(
            android.binding_output,
            repo_root.join("examples/minimal-counter/android/app/build/generated/cross-kit/uniffi")
        );
        assert_eq!(
            android.jni_libs_output,
            repo_root.join("examples/minimal-counter/dist/android/jniLibs")
        );
        assert_eq!(
            android_package.output,
            repo_root.join("examples/minimal-counter/dist/android")
        );
        assert_eq!(
            android_package.gradle_project,
            repo_root.join("examples/minimal-counter/dist/android/gradle-project")
        );
        assert_eq!(android_package.module_name, "crosskitminimalcountershared");
        assert_eq!(android_package.maven.group_id, "com.crosskit");
        assert_eq!(
            android_package.maven.artifact_id,
            "crosskitminimalcountershared"
        );
        assert_eq!(android_package.maven.version, "0.1.0");
        assert!(android_package.maven.artifact_id_explicit);
        assert_eq!(
            android_package.gradle_executable,
            repo_root.join("examples/minimal-counter/android/gradlew")
        );
    }

    #[test]
    fn maps_android_config_to_generated_and_native_paths() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"
            lib_name = "cross_kit_shared"
            metadata_bin = "metadata"

            [bindings]
            root_vm = "AppViewModel"
            container_name = "CrossKitSharedBridge"

            [android]
            package_name = "com.example.shared"
            output = "android/app/build/generated/cross-kit"
            jni_libs_output = "android/app/src/main/jniLibs"
            package_output = "dist/android"
            gradle_project_output = "dist/android/gradle-project"
            module_name = "crosskitshared"
            gradle_executable = "android/gradlew"
            java_home = "/opt/homebrew/opt/openjdk@21"
            targets = ["arm64-v8a"]
            build_mode = "debug"

            [android.maven]
            group_id = "com.example.sdk"
            artifact_id = "public-shared"
            version = "2.3.4"
            "#,
        )
        .unwrap();

        let paths =
            android_paths_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap();

        assert_eq!(paths.crate_path, PathBuf::from("/tmp/project/shared"));
        assert_eq!(paths.package_name, "com.example.shared");
        assert_eq!(
            paths.bridge_output,
            PathBuf::from("/tmp/project/android/app/build/generated/cross-kit/bridges")
        );
        assert_eq!(
            paths.binding_output,
            PathBuf::from("/tmp/project/android/app/build/generated/cross-kit/uniffi")
        );
        assert_eq!(
            paths.jni_libs_output,
            PathBuf::from("/tmp/project/android/app/src/main/jniLibs")
        );
        assert_eq!(paths.targets, vec!["arm64-v8a"]);
        assert_eq!(paths.build_mode, "debug");
        assert_eq!(paths.lib_name, "cross_kit_shared");
        assert_eq!(paths.metadata_bin, "metadata");

        let package =
            android_package_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
                .unwrap();
        assert_eq!(package.output, PathBuf::from("/tmp/project/dist/android"));
        assert_eq!(
            package.gradle_project,
            PathBuf::from("/tmp/project/dist/android/gradle-project")
        );
        assert_eq!(package.module_name, "crosskitshared");
        assert_eq!(
            package.gradle_executable,
            PathBuf::from("/tmp/project/android/gradlew")
        );
        assert_eq!(
            package.java_home,
            Some(PathBuf::from("/opt/homebrew/opt/openjdk@21"))
        );
        assert_eq!(
            package.maven,
            AndroidMavenConfig {
                group_id: "com.example.sdk".to_string(),
                artifact_id: "public-shared".to_string(),
                version: "2.3.4".to_string(),
                artifact_id_explicit: true,
            }
        );
        assert_eq!(
            package
                .bindings
                .as_ref()
                .map(|bindings| bindings.root_vm.as_str()),
            Some("AppViewModel")
        );
    }

    #[test]
    fn android_package_options_default_maven_artifact_to_module_name() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            module_name = "examplekit"
            "#,
        )
        .unwrap();

        let package =
            android_package_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
                .unwrap();

        assert_eq!(package.module_name, "examplekit");
        assert_eq!(package.maven.group_id, "com.crosskit");
        assert_eq!(package.maven.artifact_id, "examplekit");
        assert_eq!(package.maven.version, "0.1.0");
    }

    #[test]
    fn android_package_options_preserve_explicit_default_artifact_id() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            module_name = "internalshared"

            [android.maven]
            artifact_id = "crosskitshared"
            "#,
        )
        .unwrap();

        let package =
            android_package_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
                .unwrap();

        assert_eq!(package.module_name, "internalshared");
        assert_eq!(package.maven.group_id, "com.crosskit");
        assert_eq!(package.maven.artifact_id, "crosskitshared");
        assert_eq!(package.maven.version, "0.1.0");
    }

    #[test]
    fn builds_android_native_plan_and_cargo_ndk_command() {
        let paths = AndroidPaths {
            crate_path: PathBuf::from("/tmp/project/shared"),
            package_name: "com.crosskit.shared".to_string(),
            bridge_output: PathBuf::from("/tmp/project/android/generated/bridges"),
            binding_output: PathBuf::from("/tmp/project/android/generated/uniffi"),
            jni_libs_output: PathBuf::from("/tmp/project/android/jniLibs"),
            targets: vec!["arm64-v8a".to_string(), "x86_64".to_string()],
            build_mode: "release".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
        };

        let plan = android_native_plan(&paths).unwrap();
        let command = cargo_ndk_command(&paths, &plan.manifest);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            plan.manifest,
            PathBuf::from("/tmp/project/shared/Cargo.toml")
        );
        assert_eq!(
            plan.library,
            PathBuf::from("/tmp/project/android/jniLibs/arm64-v8a/libcross_kit_shared.so")
        );
        assert_eq!(command.get_program(), "cargo");
        assert_eq!(
            args,
            vec![
                "ndk",
                "-t",
                "arm64-v8a",
                "-t",
                "x86_64",
                "-o",
                "/tmp/project/android/jniLibs",
                "build",
                "--manifest-path",
                "/tmp/project/shared/Cargo.toml",
                "--release"
            ]
        );
    }

    #[test]
    fn android_native_plan_rejects_empty_targets() {
        let paths = AndroidPaths {
            crate_path: PathBuf::from("/tmp/project/shared"),
            package_name: "com.crosskit.shared".to_string(),
            bridge_output: PathBuf::from("/tmp/project/android/generated/bridges"),
            binding_output: PathBuf::from("/tmp/project/android/generated/uniffi"),
            jni_libs_output: PathBuf::from("/tmp/project/android/jniLibs"),
            targets: vec![],
            build_mode: "debug".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
        };

        let err = android_native_plan(&paths).unwrap_err();

        assert!(err.to_string().contains("[android].targets"));
    }

    #[test]
    fn rejects_invalid_android_config_before_running_build_tools() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            targets = []
            "#,
        )
        .unwrap();

        let err = android_paths_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
            .unwrap_err();
        assert!(err.to_string().contains("[android].targets"));

        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            build_mode = "fast"
            "#,
        )
        .unwrap();

        let err = android_paths_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
            .unwrap_err();
        assert!(err.to_string().contains("unsupported Android build mode"));

        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = ""

            [android]
            "#,
        )
        .unwrap();

        let err = android_paths_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
            .unwrap_err();
        assert!(err.to_string().contains("[shared].crate_path"));
    }

    #[test]
    fn generates_android_bridges_from_counter_list_metadata() {
        let config_path = repo_root().join("examples/counter-list/cross-kit.toml");

        let report = generate_android_bridges(&config_path).unwrap();

        let app_bridge = report
            .bridge_output
            .join("com/crosskit/shared/AppViewModelBridge.kt");
        let list_bridge = report
            .bridge_output
            .join("com/crosskit/shared/ListViewModelBridge.kt");
        let root_container = report
            .bridge_output
            .join("com/crosskit/shared/CrossKitSharedBridge.kt");
        let app_code = fs::read_to_string(app_bridge).unwrap();
        let list_code = fs::read_to_string(list_bridge).unwrap();
        let root_code = fs::read_to_string(root_container).unwrap();
        assert!(app_code.contains("class AppViewModelBridge(initial: Int)"));
        assert!(app_code.contains("fun clearRoute(): Unit"));
        assert!(app_code.contains("fun makeCounterVm(): CounterViewModelBridge"));
        assert!(!app_code.contains("__crossKitVm"));
        assert!(!app_code.contains("System.loadLibrary"));
        assert!(list_code.contains("SnapshotStateList<ListItem>"));
        assert!(
            list_code
                .contains("if (fromIdx !in items.indices || toIdx !in items.indices) continue")
        );
        assert!(root_code.contains("class CrossKitSharedBridge(initial: Int)"));
        assert!(root_code.contains("fun rememberCrossKitSharedBridge(initial: Int)"));
    }

    #[test]
    fn counter_list_examples_use_generated_root_container() {
        let root = repo_root();
        let ios = fs::read_to_string(
            root.join("examples/counter-list/ios/crosskit-example-ios/ContentView.swift"),
        )
        .unwrap();
        let android = fs::read_to_string(root.join(
            "examples/counter-list/android/app/src/main/java/com/example/crosskit_example_android/MainActivity.kt",
        ))
        .unwrap();
        let android_gradle =
            fs::read_to_string(root.join("examples/counter-list/android/app/build.gradle.kts"))
                .unwrap();
        let config = fs::read_to_string(root.join("examples/counter-list/cross-kit.toml")).unwrap();

        assert!(ios.contains("@StateObject private var kit = CrossKitSharedBridge(initial: 0)"));
        assert!(!ios.contains("CounterViewModelBridge(app:"));
        assert!(!ios.contains("ListViewModelBridge(app:"));
        assert!(android.contains("val kit = rememberCrossKitSharedBridge(initial = 0)"));
        assert!(!android.contains("DisposableEffect(Unit)"));
        assert!(!android.contains("appVm.makeCounterVm()"));
        assert!(!android.contains("listVm.close()"));
        assert!(config.contains("[android.maven]"));
        assert!(config.contains("group_id = \"com.crosskit\""));
        assert!(config.contains("artifact_id = \"crosskitshared\""));
        assert!(config.contains("version = \"0.1.0\""));
        assert!(android_gradle.contains("implementation(\"com.crosskit:crosskitshared:0.1.0\")"));
    }

    #[test]
    fn uniffi_binding_generation_reports_invalid_library_after_preparing_output() {
        let root = repo_root();
        let temp = temp_path("bad-uniffi");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let fake_library = temp.join("libcross_kit_shared.so");
        fs::write(&fake_library, b"not a real library").unwrap();
        let paths = AndroidPaths {
            crate_path: root.join("examples/counter-list/shared"),
            package_name: "com.crosskit.shared".to_string(),
            bridge_output: temp.join("bridges"),
            binding_output: temp.join("uniffi"),
            jni_libs_output: temp.join("jniLibs"),
            targets: vec!["arm64-v8a".to_string()],
            build_mode: "release".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
        };
        let manifest = paths.crate_path.join("Cargo.toml");

        let err = generate_uniffi_kotlin_bindings(&paths, &manifest, &fake_library).unwrap_err();

        assert!(paths.binding_output.exists());
        assert!(
            err.to_string()
                .contains("failed to generate UniFFI Kotlin bindings")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn uniffi_binding_generation_requires_existing_library() {
        let temp = temp_path("missing-uniffi");
        let _ = fs::remove_dir_all(&temp);
        let paths = AndroidPaths {
            crate_path: PathBuf::from("/tmp/project/shared"),
            package_name: "com.crosskit.shared".to_string(),
            bridge_output: temp.join("bridges"),
            binding_output: temp.join("uniffi"),
            jni_libs_output: temp.join("jniLibs"),
            targets: vec!["arm64-v8a".to_string()],
            build_mode: "release".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
        };

        let err = generate_uniffi_kotlin_bindings(
            &paths,
            Path::new("/tmp/project/shared/Cargo.toml"),
            &temp.join("jniLibs/arm64-v8a/libcross_kit_shared.so"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("expected Android library"));
    }

    #[test]
    fn android_metadata_loader_reports_failed_binary_stderr() {
        let paths = AndroidPaths {
            crate_path: PathBuf::from("/tmp/project/shared"),
            package_name: "com.crosskit.shared".to_string(),
            bridge_output: PathBuf::from("/tmp/project/bridges"),
            binding_output: PathBuf::from("/tmp/project/uniffi"),
            jni_libs_output: PathBuf::from("/tmp/project/jniLibs"),
            targets: vec!["arm64-v8a".to_string()],
            build_mode: "release".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            metadata_bin: "false".to_string(),
        };

        let err = load_vm_metadatas(&paths).unwrap_err();

        assert!(err.to_string().contains("metadata binary false failed"));
    }

    #[test]
    fn generated_dir_and_process_helpers_report_errors() {
        let temp = temp_path("generated-dir");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("stale.txt"), "old").unwrap();

        replace_generated_dir(&temp).unwrap();

        assert!(temp.exists());
        assert!(!temp.join("stale.txt").exists());
        assert_eq!(
            utf8_path(&temp).unwrap(),
            Utf8PathBuf::from_path_buf(temp.clone()).unwrap()
        );
        assert!(run_status(ProcessCommand::new("true"), "true").is_ok());
        assert!(
            run_status(ProcessCommand::new("false"), "false")
                .unwrap_err()
                .to_string()
                .contains("false failed")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn run_dispatches_command_config_errors_before_external_tools() {
        let temp = temp_path("run-dispatch");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let config_path = temp.join("cross-kit.toml");
        fs::write(
            &config_path,
            r#"
            [shared]
            crate_path = "shared"
            "#,
        )
        .unwrap();

        let ios = Cli {
            command: Command::Ios {
                command: IosCommand::Package {
                    config: config_path.clone(),
                },
            },
        };
        assert!(
            run(ios)
                .unwrap_err()
                .to_string()
                .contains("missing [ios] section")
        );

        let generated = Cli {
            command: Command::Gen {
                command: GenCommand::Bridges {
                    platform: Platform::Android,
                    config: config_path.clone(),
                },
            },
        };
        assert!(
            run(generated)
                .unwrap_err()
                .to_string()
                .contains("missing [android] section")
        );

        let native = Cli {
            command: Command::Android {
                command: AndroidCommand::BuildNative {
                    config: config_path.clone(),
                },
            },
        };
        assert!(
            run(native)
                .unwrap_err()
                .to_string()
                .contains("missing [android] section")
        );

        let package = Cli {
            command: Command::Android {
                command: AndroidCommand::Package {
                    config: config_path.clone(),
                },
            },
        };
        assert!(
            run(package)
                .unwrap_err()
                .to_string()
                .contains("missing [android] section")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn run_dispatches_successful_example_commands_when_toolchains_are_available() {
        if env::var_os("CROSS_KIT_RUN_TOOLCHAIN_TESTS").is_none() {
            return;
        }

        let root = repo_root();
        let config = root.join("examples/minimal-counter/cross-kit.toml");

        run(Cli {
            command: Command::Gen {
                command: GenCommand::Bridges {
                    platform: Platform::Android,
                    config: config.clone(),
                },
            },
        })
        .unwrap();

        if command_succeeds("cargo", &["ndk", "--version"]) {
            run(Cli {
                command: Command::Android {
                    command: AndroidCommand::BuildNative {
                        config: config.clone(),
                    },
                },
            })
            .unwrap();
            let _ = fs::remove_dir_all(root.join("examples/minimal-counter/dist/android/jniLibs"));
        }

        let java_home = Path::new("/opt/homebrew/opt/openjdk@21");
        if java_home.exists() && command_succeeds("cargo", &["ndk", "--version"]) {
            let package_config = config.with_file_name("cross-kit.package-test.toml");
            let _cleanup = RemoveFileOnDrop(package_config.clone());
            let content = fs::read_to_string(&config).unwrap().replace(
                "gradle_executable = \"android/gradlew\"",
                "gradle_executable = \"android/gradlew\"\njava_home = \"/opt/homebrew/opt/openjdk@21\"",
            );
            fs::write(&package_config, content).unwrap();
            run(Cli {
                command: Command::Android {
                    command: AndroidCommand::Package {
                        config: package_config.clone(),
                    },
                },
            })
            .unwrap();
            let _ = fs::remove_file(package_config);
        }

        if command_succeeds("xcodebuild", &["-version"]) {
            run(Cli {
                command: Command::Ios {
                    command: IosCommand::Package { config },
                },
            })
            .unwrap();
        }
    }

    fn command_succeeds(program: &str, args: &[&str]) -> bool {
        ProcessCommand::new(program)
            .args(args)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn load_android_package_options_reads_config_file() {
        let temp = temp_path("android-package-config");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let config_path = temp.join("cross-kit.toml");
        fs::write(
            &config_path,
            r#"
            [shared]
            crate_path = "shared"
            lib_name = "cross_kit_shared"
            metadata_bin = "metadata"

            [android]
            package_name = "com.example.shared"
            package_output = "dist/android"
            module_name = "examplekit"
            targets = ["arm64-v8a"]

            [android.maven]
            group_id = "com.file"
            artifact_id = "file-shared"
            version = "4.5.6"
            "#,
        )
        .unwrap();

        let options = load_android_package_options(&config_path).unwrap();

        assert_eq!(options.crate_path, temp.join("shared"));
        assert_eq!(options.package_name, "com.example.shared");
        assert_eq!(options.output, temp.join("dist/android"));
        assert_eq!(
            options.gradle_project,
            temp.join("dist/android/gradle-project")
        );
        assert_eq!(options.module_name, "examplekit");
        assert_eq!(options.targets, vec!["arm64-v8a"]);
        assert_eq!(
            options.maven,
            AndroidMavenConfig {
                group_id: "com.file".to_string(),
                artifact_id: "file-shared".to_string(),
                version: "4.5.6".to_string(),
                artifact_id_explicit: true,
            }
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn rejects_missing_ios_section() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"
            "#,
        )
        .unwrap();

        let err =
            ios_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap_err();
        assert!(err.to_string().contains("missing [ios] section"));
    }

    #[test]
    fn rejects_invalid_ios_and_android_package_values() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = ""

            [ios]
            package_name = ""
            "#,
        )
        .unwrap();
        let err =
            ios_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap_err();
        assert!(err.to_string().contains("[ios].package_name"));

        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = ""

            [ios]
            package_name = "CrossKitShared"
            "#,
        )
        .unwrap();
        let err =
            ios_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap_err();
        assert!(err.to_string().contains("[shared].crate_path"));

        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            build_mode = "fast"
            "#,
        )
        .unwrap();
        let err =
            android_package_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
                .unwrap_err();
        assert!(err.to_string().contains("unsupported Android build mode"));

        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            targets = []
            "#,
        )
        .unwrap();
        let err =
            android_package_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config)
                .unwrap_err();
        assert!(err.to_string().contains("[android].targets"));
    }

    #[test]
    fn rejects_invalid_enum_values_before_packaging() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [ios]
            package_name = "CrossKitShared"
            build_mode = "fast"
            "#,
        )
        .unwrap();

        let err =
            ios_options_from_config(Path::new("/tmp/project/cross-kit.toml"), &config).unwrap_err();
        assert!(err.to_string().contains("unsupported build mode"));
    }
}
