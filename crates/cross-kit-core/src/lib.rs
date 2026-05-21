//! Shared core constants and models for Cross-Kit crates.

#![warn(missing_docs)]

use serde::{Deserialize, Deserializer, Serialize};

/// Default Cross-Kit project configuration file name.
pub const CONFIG_FILE_NAME: &str = "cross-kit.toml";

/// Cross-Kit project configuration loaded from `cross-kit.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossKitConfig {
    /// Rust shared crate settings used by every target packager.
    pub shared: SharedConfig,
    /// Optional root graph settings for generated platform containers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bindings: Option<BindingsConfig>,
    /// Optional iOS packaging settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ios: Option<IosConfig>,
    /// Optional Android packaging settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub android: Option<AndroidConfig>,
}

impl CrossKitConfig {
    /// Parses a `cross-kit.toml` document from a string.
    ///
    /// This is the shared parser used by the CLI and tests. Validation that
    /// needs filesystem access, build tools, or target-specific context is
    /// performed by the individual packagers after parsing.
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}

/// Rust shared crate settings used to build native libraries and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedConfig {
    /// Path to the Rust crate that owns the Cross-Kit VM implementations.
    pub crate_path: String,
    /// Optional Cargo package name when it differs from the crate path name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// Optional native library base name used by target packagers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib_name: Option<String>,
    /// Cargo binary name that prints VM metadata JSON.
    #[serde(default = "default_metadata_bin")]
    pub metadata_bin: String,
}

/// Root graph settings for generated SwiftUI and Compose containers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingsConfig {
    /// Rust VM type that acts as the root of the generated graph.
    pub root_vm: String,
    /// Generated platform container name, such as `CrossKitSharedBridge`.
    pub container_name: String,
}

/// iOS packaging settings for Swift Package and native library output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IosConfig {
    /// Swift Package product/module name.
    pub package_name: String,
    /// Output directory for generated iOS artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Optional XCFramework name override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xcframework_name: Option<String>,
    /// iOS build targets requested by the package command.
    #[serde(default = "default_ios_targets")]
    pub targets: Vec<String>,
    /// Cargo build mode, usually `release` or `debug`.
    #[serde(default = "default_release")]
    pub build_mode: String,
    /// Native library kind produced for iOS, currently `static` by default.
    #[serde(default = "default_static_lib")]
    pub lib_type: String,
    /// iOS package format, currently `spm` by default.
    #[serde(default = "default_spm")]
    pub format: String,
    /// Whether Cross-Kit should generate Swift bridge classes.
    #[serde(default = "default_true")]
    pub swift_bridges: bool,
}

/// Android packaging settings for native libraries, Kotlin bindings, and AAR output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidConfig {
    /// Kotlin package name for generated Android bindings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Legacy output directory for generated Android files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Optional destination for copied JNI libraries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jni_libs_output: Option<String>,
    /// Output directory for packaged Android artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_output: Option<String>,
    /// Optional output directory for the temporary generated Gradle project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradle_project_output: Option<String>,
    /// Android library module name used inside the generated Gradle project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_name: Option<String>,
    /// Gradle executable used by the Android packager.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradle_executable: Option<String>,
    /// Java home used for generated Gradle builds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_home: Option<String>,
    /// Android ABI targets to build with `cargo-ndk`.
    #[serde(default = "default_android_targets")]
    pub targets: Vec<String>,
    /// Cargo build mode for Android native libraries.
    #[serde(default = "default_release")]
    pub build_mode: String,
    /// Maven coordinates for the generated Android AAR.
    #[serde(default)]
    pub maven: AndroidMavenConfig,
}

/// Maven coordinates used when publishing generated Android AARs locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AndroidMavenConfig {
    /// Maven group id, for example `com.crosskit`.
    #[serde(default = "default_android_maven_group_id")]
    pub group_id: String,
    /// Maven artifact id. If omitted in TOML, the CLI may derive it from the module name.
    #[serde(default = "default_android_maven_artifact_id")]
    pub artifact_id: String,
    /// Maven version for the generated AAR.
    #[serde(default = "default_android_maven_version")]
    pub version: String,
    /// Whether `artifact_id` was explicitly present in TOML.
    ///
    /// This field is skipped during serialization and lets the CLI decide
    /// whether it may replace the default artifact id with a module-derived id.
    #[serde(skip)]
    pub artifact_id_explicit: bool,
}

impl Default for AndroidMavenConfig {
    fn default() -> Self {
        Self {
            group_id: default_android_maven_group_id(),
            artifact_id: default_android_maven_artifact_id(),
            version: default_android_maven_version(),
            artifact_id_explicit: false,
        }
    }
}

impl<'de> Deserialize<'de> for AndroidMavenConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AndroidMavenConfigInput {
            #[serde(default = "default_android_maven_group_id")]
            group_id: String,
            artifact_id: Option<String>,
            #[serde(default = "default_android_maven_version")]
            version: String,
        }

        let input = AndroidMavenConfigInput::deserialize(deserializer)?;
        let artifact_id_explicit = input.artifact_id.is_some();
        Ok(Self {
            group_id: input.group_id,
            artifact_id: input
                .artifact_id
                .unwrap_or_else(default_android_maven_artifact_id),
            version: input.version,
            artifact_id_explicit,
        })
    }
}

fn default_metadata_bin() -> String {
    "ck_vm_metadata".to_string()
}

fn default_ios_targets() -> Vec<String> {
    vec!["ios".to_string(), "ios-sim".to_string()]
}

fn default_android_targets() -> Vec<String> {
    vec!["arm64-v8a".to_string(), "x86_64".to_string()]
}

fn default_android_maven_group_id() -> String {
    "com.crosskit".to_string()
}

fn default_android_maven_artifact_id() -> String {
    "crosskitshared".to_string()
}

fn default_android_maven_version() -> String {
    "0.1.0".to_string()
}

fn default_release() -> String {
    "release".to_string()
}

fn default_static_lib() -> String {
    "static".to_string()
}

fn default_spm() -> String {
    "spm".to_string()
}

fn default_true() -> bool {
    true
}

/// Current VM metadata schema version emitted by Cross-Kit macros.
pub const VM_METADATA_SCHEMA_VERSION: u32 = 1;

/// Target-independent VM metadata consumed by platform code generators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmMetadata {
    /// Metadata schema version. Generators reject unsupported versions.
    pub schema_version: u32,
    /// Rust object type annotated with `#[cross_kit::vm_bridge]`.
    pub rust_type: String,
    /// Generated platform bridge type name.
    pub bridge_name: String,
    /// Notification model used by this VM.
    pub mode: VmMode,
    /// Observer trait and callback used by state, diff-list, or event VMs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observer: Option<ObserverMetadata>,
    /// Rust state record type for `state` VMs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_type: Option<String>,
    /// Rust diff enum type for `diff_list` VMs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_type: Option<String>,
    /// Rust list item record type for `diff_list` VMs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_item_type: Option<String>,
    /// Optional parent factory that constructs this VM from a root VM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factory: Option<FactoryMetadata>,
    /// Rust VM methods visible to platform code generators.
    pub methods: Vec<MethodMetadata>,
}

impl VmMetadata {
    /// Validates the target-independent VM contract before code generation.
    ///
    /// This catches missing observer callbacks, missing state getters, invalid
    /// method shapes, unsupported schema versions, and unknown modes early so
    /// CLI/package failures point back to the VM metadata instead of generated
    /// platform source code.
    pub fn validate(&self) -> Result<(), MetadataValidationError> {
        if self.schema_version != VM_METADATA_SCHEMA_VERSION {
            return Err(MetadataValidationError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                expected: VM_METADATA_SCHEMA_VERSION,
            });
        }
        require_non_empty("rust_type", &self.rust_type)?;
        require_non_empty("bridge_name", &self.bridge_name)?;
        if let Some(factory) = &self.factory {
            require_non_empty("factory.rust_type", &factory.rust_type)?;
            require_non_empty("factory.method", &factory.method)?;
            require_non_empty("factory.bridge_name", &factory.bridge_name)?;
        }
        for method in &self.methods {
            require_non_empty("methods.name", &method.name)?;
            require_non_empty("methods.return_type", &method.return_type)?;
            for arg in &method.args {
                require_non_empty("methods.args.name", &arg.name)?;
                require_non_empty("methods.args.rust_type", &arg.rust_type)?;
            }
        }

        match self.mode {
            VmMode::State => {
                self.require_observer()?;
                require_optional("state_type", self.state_type.as_deref())?;
                self.require_observer_subscription()?;
                self.require_state_getter()?;
            }
            VmMode::DiffList => {
                self.require_observer()?;
                require_optional("diff_type", self.diff_type.as_deref())?;
                require_optional("list_item_type", self.list_item_type.as_deref())?;
                self.require_observer_subscription()?;
            }
            VmMode::Event => {
                self.require_observer()?;
                self.require_observer_subscription()?;
            }
            VmMode::Unknown => {
                return Err(MetadataValidationError::UnknownMode);
            }
        }

        Ok(())
    }

    fn require_observer(&self) -> Result<(), MetadataValidationError> {
        let observer = self
            .observer
            .as_ref()
            .ok_or(MetadataValidationError::MissingField("observer"))?;
        require_non_empty("observer.rust_type", &observer.rust_type)?;
        require_non_empty("observer.method", &observer.method)
    }

    fn require_method(&self, method_name: &'static str) -> Result<(), MetadataValidationError> {
        if self.methods.iter().any(|method| method.name == method_name) {
            Ok(())
        } else {
            Err(MetadataValidationError::MissingMethod(method_name))
        }
    }

    fn require_observer_subscription(&self) -> Result<(), MetadataValidationError> {
        self.require_method("subscribe")
    }

    fn require_state_getter(&self) -> Result<(), MetadataValidationError> {
        let method = self
            .methods
            .iter()
            .find(|method| method.name == "get_state")
            .ok_or(MetadataValidationError::MissingMethod("get_state"))?;
        if !method.args.is_empty() {
            return Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments",
            });
        }
        if method.return_type != self.state_type.as_deref().unwrap_or_default() {
            return Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "return type must match state_type",
            });
        }
        Ok(())
    }
}

/// VM change notification style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmMode {
    /// VM publishes full state snapshots through an observer callback.
    State,
    /// VM publishes list patches that platform bridges apply to observable lists.
    DiffList,
    /// Reserved mode for future one-off event streams.
    Event,
    /// Deserialized value for unknown modes so validation can report a clean error.
    #[serde(other)]
    Unknown,
}

/// Observer callback contract for a VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverMetadata {
    /// Rust observer trait type, for example `CounterObserver`.
    pub rust_type: String,
    /// Rust callback method, for example `on_state` or `on_diffs`.
    pub method: String,
}

/// Parent factory used to create a child VM bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryMetadata {
    /// Rust parent VM type that owns the factory method.
    pub rust_type: String,
    /// Rust parent method used to create the child VM.
    pub method: String,
    /// Generated bridge name for the parent VM.
    pub bridge_name: String,
}

/// Public VM method exposed to platform bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodMetadata {
    /// Rust method name in snake_case.
    pub name: String,
    /// Rust method arguments in declaration order.
    #[serde(default)]
    pub args: Vec<ArgMetadata>,
    /// Rust return type string captured from the annotated impl.
    pub return_type: String,
}

/// Public VM method argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgMetadata {
    /// Rust argument name.
    pub name: String,
    /// Rust argument type string.
    pub rust_type: String,
}

/// Validation failure for target-independent metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataValidationError {
    /// A required field was present but contained only whitespace.
    EmptyField(&'static str),
    /// A required field was absent.
    MissingField(&'static str),
    /// A required method, such as `subscribe` or `get_state`, was absent.
    MissingMethod(&'static str),
    /// A required method exists but has an unsupported signature.
    InvalidMethodShape {
        /// Method whose signature failed validation.
        method: &'static str,
        /// Human-readable reason describing the required shape.
        reason: &'static str,
    },
    /// Metadata mode could not be mapped to a supported VM mode.
    UnknownMode,
    /// Metadata schema version does not match the generator's supported version.
    UnsupportedSchemaVersion {
        /// Version found in the metadata.
        actual: u32,
        /// Version supported by this crate.
        expected: u32,
    },
}

impl std::fmt::Display for MetadataValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "metadata field `{field}` must not be empty"),
            Self::MissingField(field) => write!(f, "metadata field `{field}` is required"),
            Self::MissingMethod(method) => write!(f, "metadata method `{method}` is required"),
            Self::InvalidMethodShape { method, reason } => {
                write!(f, "metadata method `{method}` is invalid: {reason}")
            }
            Self::UnknownMode => write!(f, "metadata mode is unknown"),
            Self::UnsupportedSchemaVersion { actual, expected } => write!(
                f,
                "metadata schema version {actual} is unsupported; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for MetadataValidationError {}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), MetadataValidationError> {
    if value.trim().is_empty() {
        Err(MetadataValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn require_optional(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), MetadataValidationError> {
    let value = value.ok_or(MetadataValidationError::MissingField(field))?;
    require_non_empty(field, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_name_matches_cli_contract() {
        assert_eq!(CONFIG_FILE_NAME, "cross-kit.toml");
    }

    #[test]
    fn default_config_helpers_match_public_contract() {
        assert_eq!(default_metadata_bin(), "ck_vm_metadata");
        assert_eq!(
            default_ios_targets(),
            vec!["ios".to_string(), "ios-sim".to_string()]
        );
        assert_eq!(
            default_android_targets(),
            vec!["arm64-v8a".to_string(), "x86_64".to_string()]
        );
        assert_eq!(default_android_maven_group_id(), "com.crosskit");
        assert_eq!(default_android_maven_artifact_id(), "crosskitshared");
        assert_eq!(default_android_maven_version(), "0.1.0");
        assert_eq!(default_release(), "release");
        assert_eq!(default_static_lib(), "static");
        assert_eq!(default_spm(), "spm");
        assert!(default_true());
    }

    #[test]
    fn parses_cross_kit_toml_with_ios_package_config() {
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
            targets = ["ios", "ios-sim", "ios-sim-x86_64"]
            build_mode = "release"
            lib_type = "static"
            format = "spm"
            swift_bridges = true
            "#,
        )
        .unwrap();

        assert_eq!(config.shared.crate_path, "shared");
        assert_eq!(config.shared.package.as_deref(), Some("shared"));
        assert_eq!(config.shared.lib_name.as_deref(), Some("cross_kit_shared"));
        let bindings = config.bindings.unwrap();
        assert_eq!(bindings.root_vm, "AppViewModel");
        assert_eq!(bindings.container_name, "CrossKitSharedBridge");
        let ios = config.ios.unwrap();
        assert_eq!(ios.package_name, "CrossKitShared");
        assert_eq!(ios.output.as_deref(), Some("dist/ios"));
        assert_eq!(
            ios.targets,
            ["ios", "ios-sim", "ios-sim-x86_64"].map(str::to_string)
        );
        assert!(ios.swift_bridges);
    }

    #[test]
    fn applies_ios_config_defaults() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [ios]
            package_name = "CrossKitShared"
            "#,
        )
        .unwrap();

        assert_eq!(config.shared.metadata_bin, "ck_vm_metadata");
        let ios = config.ios.unwrap();
        assert_eq!(ios.targets, ["ios", "ios-sim"].map(str::to_string));
        assert_eq!(ios.build_mode, "release");
        assert_eq!(ios.lib_type, "static");
        assert_eq!(ios.format, "spm");
        assert!(ios.swift_bridges);
    }

    #[test]
    fn parses_android_codegen_and_native_build_config() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            package_name = "com.crosskit.shared"
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
            group_id = "com.example"
            artifact_id = "example-shared"
            version = "1.2.3"
            "#,
        )
        .unwrap();

        let android = config.android.unwrap();
        assert_eq!(android.package_name.as_deref(), Some("com.crosskit.shared"));
        assert_eq!(
            android.output.as_deref(),
            Some("android/app/build/generated/cross-kit")
        );
        assert_eq!(
            android.jni_libs_output.as_deref(),
            Some("android/app/src/main/jniLibs")
        );
        assert_eq!(android.package_output.as_deref(), Some("dist/android"));
        assert_eq!(
            android.gradle_project_output.as_deref(),
            Some("dist/android/gradle-project")
        );
        assert_eq!(android.module_name.as_deref(), Some("crosskitshared"));
        assert_eq!(
            android.gradle_executable.as_deref(),
            Some("android/gradlew")
        );
        assert_eq!(
            android.java_home.as_deref(),
            Some("/opt/homebrew/opt/openjdk@21")
        );
        assert_eq!(android.targets, ["arm64-v8a"].map(str::to_string));
        assert_eq!(android.build_mode, "debug");
        assert_eq!(android.maven.group_id, "com.example");
        assert_eq!(android.maven.artifact_id, "example-shared");
        assert_eq!(android.maven.version, "1.2.3");
        assert!(android.maven.artifact_id_explicit);
    }

    #[test]
    fn applies_android_config_defaults() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]
            "#,
        )
        .unwrap();

        let android = config.android.unwrap();
        assert_eq!(android.package_output, None);
        assert_eq!(android.gradle_project_output, None);
        assert_eq!(android.module_name, None);
        assert_eq!(android.gradle_executable, None);
        assert_eq!(android.java_home, None);
        assert_eq!(android.targets, ["arm64-v8a", "x86_64"].map(str::to_string));
        assert_eq!(android.build_mode, "release");
        assert_eq!(android.maven.group_id, "com.crosskit");
        assert_eq!(android.maven.artifact_id, "crosskitshared");
        assert_eq!(android.maven.version, "0.1.0");
        assert!(!android.maven.artifact_id_explicit);
    }

    #[test]
    fn applies_android_maven_partial_defaults() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"

            [android]

            [android.maven]
            version = "2.0.0"
            "#,
        )
        .unwrap();

        let maven = config.android.unwrap().maven;
        assert_eq!(maven.group_id, "com.crosskit");
        assert_eq!(maven.artifact_id, "crosskitshared");
        assert_eq!(maven.version, "2.0.0");
        assert!(!maven.artifact_id_explicit);
    }

    #[test]
    fn parses_and_validates_state_metadata() {
        let metadata: VmMetadata = serde_json::from_value(serde_json::json!({
            "schema_version": VM_METADATA_SCHEMA_VERSION,
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
                    "name": "get_state",
                    "args": [],
                    "return_type": "CounterState"
                },
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn CounterObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "increment_by",
                    "args": [{"name": "delta", "rust_type": "i32"}],
                    "return_type": "CounterState"
                }
            ]
        }))
        .unwrap();

        assert_eq!(metadata.mode, VmMode::State);
        assert_eq!(
            metadata.factory.as_ref().unwrap().bridge_name,
            "AppViewModelBridge"
        );
        metadata.validate().unwrap();
    }

    #[test]
    fn parses_diff_list_metadata_with_collection_types() {
        let metadata: VmMetadata = serde_json::from_value(serde_json::json!({
            "schema_version": VM_METADATA_SCHEMA_VERSION,
            "rust_type": "ListViewModel",
            "bridge_name": "ListViewModelBridge",
            "mode": "diff_list",
            "observer": {
                "rust_type": "ListObserver",
                "method": "on_diffs"
            },
            "diff_type": "ListDiff",
            "list_item_type": "ListItem",
            "methods": [
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn ListObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "apply_diffs",
                    "args": [{"name": "diffs", "rust_type": "Vec<ListDiff>"}],
                    "return_type": "bool"
                },
                {
                    "name": "find_item",
                    "args": [{"name": "id", "rust_type": "i64"}],
                    "return_type": "Option<ListItem>"
                }
            ]
        }))
        .unwrap();

        assert_eq!(metadata.mode, VmMode::DiffList);
        assert_eq!(
            metadata.methods[2].args[0].rust_type, "i64",
            "record and enum type names are passed through as Rust type strings"
        );
        metadata.validate().unwrap();
    }

    #[test]
    fn rejects_state_metadata_without_required_get_state_method() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn CounterObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::MissingMethod("get_state"))
        );
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION + 1,
            rust_type: "EventViewModel".to_string(),
            bridge_name: "EventViewModelBridge".to_string(),
            mode: VmMode::Event,
            observer: Some(ObserverMetadata {
                rust_type: "EventObserver".to_string(),
                method: "on_event".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn EventObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::UnsupportedSchemaVersion {
                actual: VM_METADATA_SCHEMA_VERSION + 1,
                expected: VM_METADATA_SCHEMA_VERSION
            })
        );
    }

    #[test]
    fn validates_event_metadata_with_observer() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "EventViewModel".to_string(),
            bridge_name: "EventViewModelBridge".to_string(),
            mode: VmMode::Event,
            observer: Some(ObserverMetadata {
                rust_type: "EventObserver".to_string(),
                method: "on_event".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn EventObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        metadata.validate().unwrap();
    }

    #[test]
    fn rejects_unknown_mode_and_missing_observer() {
        let unknown_mode = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "UnknownViewModel".to_string(),
            bridge_name: "UnknownViewModelBridge".to_string(),
            mode: VmMode::Unknown,
            observer: Some(ObserverMetadata {
                rust_type: "UnknownObserver".to_string(),
                method: "on_unknown".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            unknown_mode.validate(),
            Err(MetadataValidationError::UnknownMode)
        );

        let missing_observer = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: None,
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            missing_observer.validate(),
            Err(MetadataValidationError::MissingField("observer"))
        );
    }

    #[test]
    fn rejects_empty_required_fields() {
        let empty_root_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: " ".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "CounterState".to_string(),
                },
            ],
        };
        assert_eq!(
            empty_root_field.validate(),
            Err(MetadataValidationError::EmptyField("rust_type"))
        );

        let empty_optional_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some(" ".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            empty_optional_field.validate(),
            Err(MetadataValidationError::EmptyField("diff_type"))
        );
    }

    #[test]
    fn rejects_empty_factory_and_method_fields() {
        let empty_factory_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: Some(FactoryMetadata {
                rust_type: "AppViewModel".to_string(),
                method: " ".to_string(),
                bridge_name: "AppViewModelBridge".to_string(),
            }),
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn CounterObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };
        assert_eq!(
            empty_factory_field.validate(),
            Err(MetadataValidationError::EmptyField("factory.method"))
        );

        let empty_method_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: " ".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };
        assert_eq!(
            empty_method_field.validate(),
            Err(MetadataValidationError::EmptyField(
                "methods.args.rust_type"
            ))
        );
    }

    #[test]
    fn rejects_observer_vm_without_subscribe_method() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: vec![MethodMetadata {
                name: "apply_diffs".to_string(),
                args: Vec::new(),
                return_type: "bool".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::MissingMethod("subscribe"))
        );
    }

    #[test]
    fn rejects_state_getter_with_wrong_shape() {
        let wrong_args = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: vec![ArgMetadata {
                        name: "id".to_string(),
                        rust_type: "i64".to_string(),
                    }],
                    return_type: "CounterState".to_string(),
                },
            ],
        };
        assert_eq!(
            wrong_args.validate(),
            Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments"
            })
        );

        let wrong_return = VmMetadata {
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "OtherState".to_string(),
                },
            ],
            ..wrong_args
        };
        assert_eq!(
            wrong_return.validate(),
            Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "return type must match state_type"
            })
        );
    }

    #[test]
    fn validation_errors_have_actionable_messages() {
        assert_eq!(
            MetadataValidationError::EmptyField("rust_type").to_string(),
            "metadata field `rust_type` must not be empty"
        );
        assert_eq!(
            MetadataValidationError::MissingField("observer").to_string(),
            "metadata field `observer` is required"
        );
        assert_eq!(
            MetadataValidationError::MissingMethod("get_state").to_string(),
            "metadata method `get_state` is required"
        );
        assert_eq!(
            (MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments",
            })
            .to_string(),
            "metadata method `get_state` is invalid: must not accept arguments"
        );
        assert_eq!(
            MetadataValidationError::UnknownMode.to_string(),
            "metadata mode is unknown"
        );
        assert_eq!(
            (MetadataValidationError::UnsupportedSchemaVersion {
                actual: 2,
                expected: 1,
            })
            .to_string(),
            "metadata schema version 2 is unsupported; expected 1"
        );
    }

    #[test]
    fn fixture_ir_contracts_parse_and_validate() {
        let fixture = include_str!("../../../fixtures/metadata/counter-list.json");
        let metadatas: Vec<VmMetadata> = serde_json::from_str(fixture).unwrap();

        for metadata in metadatas {
            metadata.validate().unwrap();
        }
    }
}
