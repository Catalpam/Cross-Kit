use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use cross_kit_core::{AndroidMavenConfig, BindingsConfig, VmMetadata};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidPackageOptions {
    pub crate_path: PathBuf,
    pub package_name: String,
    pub lib_name: String,
    pub output: PathBuf,
    pub gradle_project: PathBuf,
    pub module_name: String,
    pub gradle_executable: PathBuf,
    pub java_home: Option<PathBuf>,
    pub targets: Vec<String>,
    pub build_mode: String,
    pub metadata_bin: String,
    pub maven: AndroidMavenConfig,
    pub bindings: Option<BindingsConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidPackageReport {
    pub aar: PathBuf,
    pub maven_repo: PathBuf,
    pub gradle_project: PathBuf,
    pub module_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AndroidPackageLayout {
    gradle_project: PathBuf,
    module_dir: PathBuf,
    kotlin_dir: PathBuf,
    jni_libs_dir: PathBuf,
    native_output_dir: PathBuf,
    maven_repo: PathBuf,
    aar_build_output: PathBuf,
    aar_final_output: PathBuf,
}

pub fn package_android(options: &AndroidPackageOptions) -> Result<AndroidPackageReport> {
    validate_options(options)?;
    let layout = package_layout(options);
    let metadatas = load_vm_metadatas(options)?;

    replace_dir(&layout.gradle_project)?;
    write_gradle_project(options, &layout)?;
    write_kotlin_sources(options, &layout, &metadatas)?;
    build_native_libraries(options, &layout)?;
    generate_uniffi_bindings(options, &layout)?;
    run_gradle_assemble(options, &layout)?;
    run_gradle_publish(options, &layout)?;
    copy_aar(&layout)?;

    Ok(AndroidPackageReport {
        aar: layout.aar_final_output,
        maven_repo: layout.maven_repo,
        gradle_project: layout.gradle_project,
        module_dir: layout.module_dir,
    })
}

fn validate_options(options: &AndroidPackageOptions) -> Result<()> {
    if options.crate_path.as_os_str().is_empty() {
        bail!("crate_path must not be empty");
    }
    if options.package_name.trim().is_empty() {
        bail!("package_name must not be empty");
    }
    if options.lib_name.trim().is_empty() {
        bail!("lib_name must not be empty");
    }
    if options.module_name.trim().is_empty() {
        bail!("module_name must not be empty");
    }
    if options.targets.is_empty()
        || options
            .targets
            .iter()
            .any(|target| target.trim().is_empty())
    {
        bail!("targets must contain at least one non-empty Android ABI");
    }
    if options.build_mode != "debug" && options.build_mode != "release" {
        bail!(
            "unsupported Android build mode '{}'; expected 'debug' or 'release'",
            options.build_mode
        );
    }
    if options.maven.group_id.trim().is_empty() {
        bail!("android.maven.group_id must not be empty");
    }
    if options.maven.artifact_id.trim().is_empty() {
        bail!("android.maven.artifact_id must not be empty");
    }
    if options.maven.version.trim().is_empty() {
        bail!("android.maven.version must not be empty");
    }
    Ok(())
}

fn package_layout(options: &AndroidPackageOptions) -> AndroidPackageLayout {
    let module_dir = options.gradle_project.join(&options.module_name);
    let aar_name = format!("{}-{}.aar", options.module_name, options.build_mode);
    AndroidPackageLayout {
        gradle_project: options.gradle_project.clone(),
        kotlin_dir: module_dir.join("src/main/kotlin"),
        jni_libs_dir: module_dir.join("src/main/jniLibs"),
        native_output_dir: options.output.join("native"),
        maven_repo: options.output.join("maven"),
        aar_build_output: module_dir.join("build/outputs/aar").join(&aar_name),
        aar_final_output: options.output.join(aar_name),
        module_dir,
    }
}

fn load_vm_metadatas(options: &AndroidPackageOptions) -> Result<Vec<VmMetadata>> {
    let manifest = options.crate_path.join("Cargo.toml");
    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--bin")
        .arg(&options.metadata_bin)
        .output()
        .with_context(|| format!("failed to run metadata binary {}", options.metadata_bin))?;
    if !output.status.success() {
        bail!(
            "metadata binary {} failed:\n{}",
            options.metadata_bin,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let values: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "failed to parse metadata JSON from {}",
                options.metadata_bin
            )
        })?;
    values
        .into_iter()
        .map(|value| {
            let ir = value.get("ir").cloned().unwrap_or(value);
            serde_json::from_value(ir).context("failed to parse VM metadata IR")
        })
        .collect()
}

fn write_gradle_project(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    fs::create_dir_all(layout.module_dir.join("src/main"))?;
    fs::write(
        layout.gradle_project.join("settings.gradle.kts"),
        settings_gradle(&options.module_name),
    )?;
    fs::write(
        layout.gradle_project.join("gradle.properties"),
        gradle_properties(),
    )?;
    fs::write(
        layout.gradle_project.join("build.gradle.kts"),
        root_gradle(),
    )?;
    fs::write(
        layout.module_dir.join("build.gradle.kts"),
        module_gradle(options, layout),
    )?;
    fs::write(
        layout.module_dir.join("src/main/AndroidManifest.xml"),
        android_manifest(&options.package_name),
    )?;
    fs::write(layout.module_dir.join("consumer-rules.pro"), "")?;
    Ok(())
}

fn write_kotlin_sources(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
    metadatas: &[VmMetadata],
) -> Result<()> {
    replace_dir(&layout.kotlin_dir)?;
    for metadata in metadatas {
        let files = cross_kit_codegen::generate_kotlin_bridge(metadata, &options.package_name)?;
        for file in files.files {
            let path = layout.kotlin_dir.join(file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, file.contents)?;
        }
    }
    if let Some(bindings) = &options.bindings {
        let files = cross_kit_codegen::generate_kotlin_root_container(
            metadatas,
            bindings,
            &options.package_name,
        )?;
        for file in files.files {
            let path = layout.kotlin_dir.join(file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, file.contents)?;
        }
    }
    Ok(())
}

fn build_native_libraries(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    let manifest = options.crate_path.join("Cargo.toml");
    replace_dir(&layout.native_output_dir)?;
    let mut command = cargo_ndk_command(options, &manifest, &layout.native_output_dir);
    run_status(&mut command, "cargo ndk build")?;
    replace_dir(&layout.jni_libs_dir)?;
    copy_native_libraries(options, layout)
}

fn copy_native_libraries(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    for target in &options.targets {
        let source = layout
            .native_output_dir
            .join(target)
            .join(format!("lib{}.so", options.lib_name));
        let dest = layout
            .jni_libs_dir
            .join(target)
            .join(format!("lib{}.so", options.lib_name));
        if !source.exists() {
            bail!("expected Android library at {}", source.display());
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &dest).with_context(|| {
            format!("failed to copy {} to {}", source.display(), dest.display())
        })?;
    }
    Ok(())
}

fn generate_uniffi_bindings(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    let first_target = options
        .targets
        .first()
        .ok_or_else(|| anyhow::anyhow!("targets must not be empty"))?;
    let library = layout
        .native_output_dir
        .join(first_target)
        .join(format!("lib{}.so", options.lib_name));
    if !library.exists() {
        bail!("expected Android library at {}", library.display());
    }
    let manifest = options.crate_path.join("Cargo.toml");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest)
        .exec()
        .context("failed to load Cargo metadata for UniFFI")?;
    let config_supplier = uniffi_bindgen::cargo_metadata::CrateConfigSupplier::from(metadata);
    let library = utf8_path(&library)?;
    let out_dir = utf8_path(&layout.kotlin_dir)?;
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

fn run_gradle_assemble(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    run_gradle_task(
        options,
        layout,
        &format!("assemble{}", gradle_variant(options)),
    )
}

fn run_gradle_publish(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
) -> Result<()> {
    run_gradle_task(
        options,
        layout,
        &format!(
            "publish{}PublicationToLocalCrossKitRepository",
            gradle_variant(options)
        ),
    )
}

fn run_gradle_task(
    options: &AndroidPackageOptions,
    layout: &AndroidPackageLayout,
    task_name: &str,
) -> Result<()> {
    if !options.gradle_executable.exists() {
        bail!(
            "Gradle executable does not exist: {}",
            options.gradle_executable.display()
        );
    }
    let task = format!(":{}:{task_name}", options.module_name);
    let mut command = Command::new(&options.gradle_executable);
    command.arg("-p").arg(&layout.gradle_project).arg(task);
    if let Some(java_home) = &options.java_home {
        command.env("JAVA_HOME", java_home);
    }
    run_status(&mut command, task_name)
}

fn gradle_variant(options: &AndroidPackageOptions) -> &'static str {
    if options.build_mode == "debug" {
        "Debug"
    } else {
        "Release"
    }
}

fn copy_aar(layout: &AndroidPackageLayout) -> Result<()> {
    if !layout.aar_build_output.exists() {
        bail!(
            "expected AAR at {} after Gradle build",
            layout.aar_build_output.display()
        );
    }
    fs::create_dir_all(&layout.aar_final_output.parent().unwrap_or(Path::new(".")))?;
    fs::copy(&layout.aar_build_output, &layout.aar_final_output).with_context(|| {
        format!(
            "failed to copy {} to {}",
            layout.aar_build_output.display(),
            layout.aar_final_output.display()
        )
    })?;
    Ok(())
}

fn cargo_ndk_command(options: &AndroidPackageOptions, manifest: &Path, output: &Path) -> Command {
    let mut command = Command::new("cargo");
    command.arg("ndk");
    for target in &options.targets {
        command.arg("-t").arg(target);
    }
    command
        .arg("-o")
        .arg(output)
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest);
    if options.build_mode == "release" {
        command.arg("--release");
    }
    command
}

fn settings_gradle(module_name: &str) -> String {
    format!(
        r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}
dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "cross-kit-android-package"
include(":{module_name}")
"#
    )
}

fn root_gradle() -> &'static str {
    r#"plugins {
    id("com.android.library") version "8.6.1" apply false
    id("org.jetbrains.kotlin.android") version "1.9.0" apply false
}
"#
}

fn gradle_properties() -> &'static str {
    "android.useAndroidX=true\nandroid.nonTransitiveRClass=true\n"
}

fn module_gradle(options: &AndroidPackageOptions, layout: &AndroidPackageLayout) -> String {
    let maven_repo = gradle_uri(&layout.maven_repo);
    format!(
        r#"plugins {{
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
}}

group = "{group_id}"
version = "{version}"

android {{
    namespace = "{namespace}"
    compileSdk = 34

    defaultConfig {{
        minSdk = 23
        consumerProguardFiles("consumer-rules.pro")
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }}
    kotlinOptions {{
        jvmTarget = "1.8"
    }}
    buildFeatures {{
        compose = true
    }}
    composeOptions {{
        kotlinCompilerExtensionVersion = "1.5.1"
    }}
}}

dependencies {{
    api(platform("androidx.compose:compose-bom:2024.04.01"))
    api("androidx.compose.runtime:runtime")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}}

publishing {{
    repositories {{
        maven {{
            name = "localCrossKit"
            url = uri("{maven_repo}")
        }}
    }}
    publications {{
        create<MavenPublication>("{variant}") {{
            afterEvaluate {{
                from(components["{variant}"])
            }}
            artifactId = "{artifact_id}"
        }}
    }}
}}
"#,
        namespace = options.package_name,
        group_id = &options.maven.group_id,
        version = &options.maven.version,
        maven_repo = maven_repo,
        variant = options.build_mode,
        artifact_id = &options.maven.artifact_id
    )
}

fn gradle_uri(path: &Path) -> String {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    path.to_string_lossy().replace('\\', "\\\\")
}

fn android_manifest(package_name: &str) -> String {
    let _ = package_name;
    r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-sdk android:minSdkVersion="23" />
</manifest>
"#
    .to_string()
}

fn replace_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))
}

fn utf8_path(path: &Path) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|path| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn run_status(command: &mut Command, name: &str) -> Result<()> {
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
    use std::{env, process};

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "cross-kit-packager-android-test-{}-{name}",
            process::id()
        ))
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn options() -> AndroidPackageOptions {
        AndroidPackageOptions {
            crate_path: PathBuf::from("/tmp/project/shared"),
            package_name: "com.crosskit.shared".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            output: PathBuf::from("/tmp/project/dist/android"),
            gradle_project: PathBuf::from("/tmp/project/dist/android/gradle-project"),
            module_name: "crosskitshared".to_string(),
            gradle_executable: PathBuf::from("/tmp/project/android/gradlew"),
            java_home: None,
            targets: vec!["arm64-v8a".to_string(), "x86_64".to_string()],
            build_mode: "release".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
            maven: AndroidMavenConfig::default(),
            bindings: None,
        }
    }

    #[test]
    fn computes_package_layout_and_cargo_ndk_command() {
        let options = options();
        let layout = package_layout(&options);
        let manifest = options.crate_path.join("Cargo.toml");
        let command = cargo_ndk_command(&options, &manifest, &layout.native_output_dir);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            layout.aar_final_output,
            PathBuf::from("/tmp/project/dist/android/crosskitshared-release.aar")
        );
        assert_eq!(
            layout.maven_repo,
            PathBuf::from("/tmp/project/dist/android/maven")
        );
        assert_eq!(
            layout.kotlin_dir,
            PathBuf::from(
                "/tmp/project/dist/android/gradle-project/crosskitshared/src/main/kotlin"
            )
        );
        assert_eq!(
            args,
            vec![
                "ndk",
                "-t",
                "arm64-v8a",
                "-t",
                "x86_64",
                "-o",
                "/tmp/project/dist/android/native",
                "build",
                "--manifest-path",
                "/tmp/project/shared/Cargo.toml",
                "--release"
            ]
        );
    }

    #[test]
    fn debug_build_mode_uses_debug_gradle_variant_and_aar_name() {
        let mut options = options();
        options.build_mode = "debug".to_string();
        let layout = package_layout(&options);

        assert_eq!(
            layout.aar_final_output,
            PathBuf::from("/tmp/project/dist/android/crosskitshared-debug.aar")
        );
        assert_eq!(gradle_variant(&options), "Debug");
    }

    #[test]
    fn gradle_uri_absolutizes_relative_maven_repo_paths() {
        let uri = gradle_uri(Path::new("dist/android/maven"));

        assert!(Path::new(&uri).is_absolute());
        assert!(uri.ends_with("dist/android/maven"));
    }

    #[test]
    fn writes_gradle_project_with_fixed_android_dependencies() {
        let mut options = options();
        let temp = temp_path("gradle-project");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);

        write_gradle_project(&options, &layout).unwrap();

        let settings =
            fs::read_to_string(layout.gradle_project.join("settings.gradle.kts")).unwrap();
        let properties =
            fs::read_to_string(layout.gradle_project.join("gradle.properties")).unwrap();
        let module = fs::read_to_string(layout.module_dir.join("build.gradle.kts")).unwrap();
        let manifest =
            fs::read_to_string(layout.module_dir.join("src/main/AndroidManifest.xml")).unwrap();
        assert!(settings.contains("include(\":crosskitshared\")"));
        assert!(properties.contains("android.useAndroidX=true"));
        assert!(module.contains("id(\"com.android.library\")"));
        assert!(module.contains("id(\"maven-publish\")"));
        assert!(module.contains("group = \"com.crosskit\""));
        assert!(module.contains("version = \"0.1.0\""));
        assert!(module.contains("artifactId = \"crosskitshared\""));
        assert!(module.contains("publish"));
        assert!(module.contains("compose = true"));
        assert!(module.contains("kotlinCompilerExtensionVersion = \"1.5.1\""));
        assert!(module.contains("api(\"androidx.compose.runtime:runtime\")"));
        assert!(module.contains("net.java.dev.jna:jna:5.14.0@aar"));
        assert!(manifest.contains("<manifest"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn writes_gradle_project_with_configured_maven_coordinates() {
        let mut options = options();
        options.module_name = "internalshared".to_string();
        options.maven = AndroidMavenConfig {
            group_id: "com.example.sdk".to_string(),
            artifact_id: "public-shared".to_string(),
            version: "2.3.4".to_string(),
            artifact_id_explicit: true,
        };
        let temp = temp_path("gradle-project-maven");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);

        write_gradle_project(&options, &layout).unwrap();

        let settings =
            fs::read_to_string(layout.gradle_project.join("settings.gradle.kts")).unwrap();
        let module = fs::read_to_string(layout.module_dir.join("build.gradle.kts")).unwrap();
        assert!(settings.contains("include(\":internalshared\")"));
        assert!(module.contains("group = \"com.example.sdk\""));
        assert!(module.contains("version = \"2.3.4\""));
        assert!(module.contains("artifactId = \"public-shared\""));
        assert!(!module.contains("group = \"com.crosskit\""));
        assert!(!module.contains("version = \"0.1.0\""));
        assert!(!module.contains("artifactId = \"internalshared\""));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn writes_kotlin_sources_into_package_path() {
        let mut options = options();
        let temp = temp_path("kotlin-sources");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);
        let metadata = VmMetadata {
            schema_version: 1,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: cross_kit_core::VmMode::State,
            observer: Some(cross_kit_core::ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![
                cross_kit_core::MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![cross_kit_core::ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                cross_kit_core::MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "CounterState".to_string(),
                },
                cross_kit_core::MethodMetadata {
                    name: "unsubscribe".to_string(),
                    args: vec![cross_kit_core::ArgMetadata {
                        name: "id".to_string(),
                        rust_type: "i64".to_string(),
                    }],
                    return_type: "unit".to_string(),
                },
                cross_kit_core::MethodMetadata {
                    name: "__cross_kit_probe".to_string(),
                    args: Vec::new(),
                    return_type: "unit".to_string(),
                },
                cross_kit_core::MethodMetadata {
                    name: "on_state".to_string(),
                    args: Vec::new(),
                    return_type: "unit".to_string(),
                },
                cross_kit_core::MethodMetadata {
                    name: "increment_by".to_string(),
                    args: vec![cross_kit_core::ArgMetadata {
                        name: "delta_value".to_string(),
                        rust_type: "i32".to_string(),
                    }],
                    return_type: "CounterState".to_string(),
                },
            ],
        };

        write_kotlin_sources(&options, &layout, &[metadata]).unwrap();

        let bridge = layout
            .kotlin_dir
            .join("com/crosskit/shared/CounterViewModelBridge.kt");
        assert!(bridge.exists());
        let bridge_code = fs::read_to_string(bridge).unwrap();
        assert!(bridge_code.contains("vm.close()"));
        assert!(bridge_code.contains("fun incrementBy(deltaValue: Int): CounterState"));
        assert!(!bridge_code.contains("fun getState"));
        assert!(!bridge_code.contains("fun subscribe"));
        assert!(!bridge_code.contains("fun unsubscribe"));
        assert!(!bridge_code.contains("fun __crossKitProbe"));
        assert!(!bridge_code.contains("/** Calls the Rust VM `on_state` action. */"));
        assert!(!bridge_code.contains("vm.onState("));
        assert!(!bridge_code.contains("\n            vm."));
        assert!(!bridge_code.contains("\n            return vm."));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn writes_kotlin_root_container_when_bindings_are_configured() {
        let mut options = options();
        options.bindings = Some(cross_kit_core::BindingsConfig {
            root_vm: "AppViewModel".to_string(),
            container_name: "CrossKitSharedBridge".to_string(),
        });
        let temp = temp_path("kotlin-root-container");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);

        write_kotlin_sources(
            &options,
            &layout,
            &[android_app_metadata(), android_counter_metadata()],
        )
        .unwrap();

        let root_file = layout
            .kotlin_dir
            .join("com/crosskit/shared/CrossKitSharedBridge.kt");
        let counter_file = layout
            .kotlin_dir
            .join("com/crosskit/shared/CounterViewModelBridge.kt");
        let root_code = fs::read_to_string(root_file).unwrap();
        assert!(counter_file.exists());
        assert!(root_code.contains("class CrossKitSharedBridge(initial: Int) : AutoCloseable"));
        assert!(root_code.contains("val app: AppViewModelBridge = AppViewModelBridge(initial)"));
        assert!(root_code.contains("val counter: CounterViewModelBridge = app.makeCounterVm()"));
        assert!(root_code.contains("fun rememberCrossKitSharedBridge(initial: Int)"));
        assert!(root_code.contains("onDispose { kit.close() }"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn validates_options_before_running_tools() {
        let mut invalid = options();
        invalid.crate_path = PathBuf::new();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("crate_path")
        );

        let mut invalid = options();
        invalid.package_name = " ".to_string();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("package_name")
        );

        let mut invalid = options();
        invalid.lib_name = "".to_string();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("lib_name")
        );

        let mut invalid = options();
        invalid.targets.clear();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("targets")
        );

        let mut invalid = options();
        invalid.build_mode = "fast".to_string();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("unsupported Android build mode")
        );

        let mut invalid = options();
        invalid.module_name = "".to_string();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("module_name")
        );

        let mut invalid = options();
        invalid.maven.group_id = " ".to_string();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("android.maven.group_id")
        );

        let mut invalid = options();
        invalid.maven.artifact_id.clear();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("android.maven.artifact_id")
        );

        let mut invalid = options();
        invalid.maven.version.clear();
        assert!(
            validate_options(&invalid)
                .unwrap_err()
                .to_string()
                .contains("android.maven.version")
        );
    }

    #[test]
    fn metadata_loader_reports_missing_binary() {
        let mut options = options();
        options.crate_path = repo_root().join("examples/counter-list/shared");
        options.metadata_bin = "does_not_exist".to_string();

        let err = load_vm_metadatas(&options).unwrap_err();

        assert!(
            err.to_string()
                .contains("metadata binary does_not_exist failed")
        );
    }

    #[test]
    fn native_and_uniffi_steps_report_missing_inputs() {
        let mut options = options();
        let temp = temp_path("missing-native");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);
        fs::create_dir_all(&layout.native_output_dir).unwrap();

        let err = generate_uniffi_bindings(&options, &layout).unwrap_err();
        assert!(err.to_string().contains("expected Android library"));

        let err = copy_native_libraries(&options, &layout).unwrap_err();
        assert!(err.to_string().contains("expected Android library"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn gradle_and_filesystem_helpers_report_errors_and_copy_aar() {
        let mut options = options();
        let temp = temp_path("helpers");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        options.gradle_executable = temp.join("missing-gradlew");
        let layout = package_layout(&options);

        let err = run_gradle_assemble(&options, &layout).unwrap_err();
        assert!(err.to_string().contains("Gradle executable does not exist"));

        fs::create_dir_all(layout.aar_build_output.parent().unwrap()).unwrap();
        fs::write(&layout.aar_build_output, b"aar").unwrap();
        copy_aar(&layout).unwrap();
        assert_eq!(fs::read(&layout.aar_final_output).unwrap(), b"aar");

        let stale = temp.join("stale-dir");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("old.txt"), "old").unwrap();
        replace_dir(&stale).unwrap();
        assert!(stale.exists());
        assert!(!stale.join("old.txt").exists());

        assert!(
            run_status(&mut Command::new("false"), "false")
                .unwrap_err()
                .to_string()
                .contains("false failed")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn copy_aar_reports_missing_gradle_output() {
        let mut options = options();
        let temp = temp_path("missing-aar");
        let _ = fs::remove_dir_all(&temp);
        options.output = temp.join("dist");
        options.gradle_project = temp.join("dist/gradle-project");
        let layout = package_layout(&options);

        let err = copy_aar(&layout).unwrap_err();

        assert!(err.to_string().contains("expected AAR"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn packages_counter_list_aar_when_android_environment_is_available() {
        let repo_root = repo_root();
        let java_home = PathBuf::from("/opt/homebrew/opt/openjdk@21");
        let gradlew = repo_root.join("examples/counter-list/android/gradlew");
        let cargo_ndk_available = Command::new("cargo")
            .arg("ndk")
            .arg("--version")
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !java_home.exists() || !gradlew.exists() || !cargo_ndk_available {
            return;
        }

        let temp = temp_path("package-aar");
        let _ = fs::remove_dir_all(&temp);
        let options = AndroidPackageOptions {
            crate_path: repo_root.join("examples/counter-list/shared"),
            package_name: "com.crosskit.shared".to_string(),
            lib_name: "cross_kit_shared".to_string(),
            output: temp.join("dist/android"),
            gradle_project: temp.join("dist/android/gradle-project"),
            module_name: "crosskitshared".to_string(),
            gradle_executable: gradlew,
            java_home: Some(java_home),
            targets: vec!["arm64-v8a".to_string(), "x86_64".to_string()],
            build_mode: "release".to_string(),
            metadata_bin: "ck_vm_metadata".to_string(),
            maven: AndroidMavenConfig {
                group_id: "com.example.crosskit".to_string(),
                artifact_id: "public-shared".to_string(),
                version: "9.8.7".to_string(),
                artifact_id_explicit: true,
            },
            bindings: None,
        };

        let report = package_android(&options).unwrap();

        assert!(report.aar.exists());
        assert!(
            report
                .maven_repo
                .join("com/example/crosskit/public-shared/9.8.7/public-shared-9.8.7.pom")
                .exists()
        );
        let pom = fs::read_to_string(
            report
                .maven_repo
                .join("com/example/crosskit/public-shared/9.8.7/public-shared-9.8.7.pom"),
        )
        .unwrap();
        let module = fs::read_to_string(
            report
                .maven_repo
                .join("com/example/crosskit/public-shared/9.8.7/public-shared-9.8.7.module"),
        )
        .unwrap();
        assert!(pom.contains("<groupId>com.example.crosskit</groupId>"));
        assert!(pom.contains("<artifactId>public-shared</artifactId>"));
        assert!(pom.contains("<version>9.8.7</version>"));
        assert!(pom.contains("<artifactId>jna</artifactId>"));
        assert!(pom.contains("<type>aar</type>"));
        assert!(pom.contains("<artifactId>runtime</artifactId>"));
        assert!(module.contains("\"group\": \"com.example.crosskit\""));
        assert!(module.contains("\"module\": \"public-shared\""));
        assert!(module.contains("\"version\": \"9.8.7\""));
        assert!(module.contains("\"name\": \"jna\""));
        assert!(module.contains("\"type\": \"aar\""));
        assert!(
            report
                .module_dir
                .join("src/main/kotlin/com/crosskit/shared/AppViewModelBridge.kt")
                .exists()
        );
        assert!(
            report
                .module_dir
                .join("src/main/jniLibs/arm64-v8a/libcross_kit_shared.so")
                .exists()
        );
        assert!(
            report
                .module_dir
                .join("src/main/jniLibs/x86_64/libcross_kit_shared.so")
                .exists()
        );
        let _ = fs::remove_dir_all(&temp);
    }

    fn android_app_metadata() -> VmMetadata {
        serde_json::from_value(serde_json::json!({
            "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
            "rust_type": "AppViewModel",
            "bridge_name": "AppViewModelBridge",
            "mode": "state",
            "observer": {"rust_type": "AppObserver", "method": "on_state"},
            "state_type": "AppState",
            "methods": [
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn AppObserver>"}],
                    "return_type": "i64"
                },
                {"name": "get_state", "args": [], "return_type": "AppState"},
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

    fn android_counter_metadata() -> VmMetadata {
        serde_json::from_value(serde_json::json!({
            "schema_version": cross_kit_core::VM_METADATA_SCHEMA_VERSION,
            "rust_type": "CounterViewModel",
            "bridge_name": "CounterViewModelBridge",
            "mode": "state",
            "observer": {"rust_type": "CounterObserver", "method": "on_state"},
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
                {"name": "get_state", "args": [], "return_type": "CounterState"}
            ]
        }))
        .unwrap()
    }
}
