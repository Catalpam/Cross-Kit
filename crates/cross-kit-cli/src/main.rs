use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use cross_kit_core::{CONFIG_FILE_NAME, CrossKitConfig};
use cross_kit_packager_ios::{BuildMode, IosPackageOptions, LibType, PackageFormat};
use std::fs;
use std::path::{Path, PathBuf};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ios_config_to_packager_options_with_relative_paths() {
        let config = CrossKitConfig::from_toml_str(
            r#"
            [shared]
            crate_path = "shared"
            package = "shared"
            lib_name = "cross_kit_shared"
            metadata_bin = "ck_vm_metadata"

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
    }

    #[test]
    fn loads_counter_list_example_config_after_directory_migration() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config_path = repo_root.join("examples/counter-list/cross-kit.toml");

        let options = load_ios_options(&config_path).unwrap();

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
