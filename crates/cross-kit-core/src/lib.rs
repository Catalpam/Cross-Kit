//! Shared core constants and models for Cross-Kit crates.

/// Default Cross-Kit project configuration file name.
pub const CONFIG_FILE_NAME: &str = "cross-kit.toml";

#[cfg(test)]
mod tests {
    use super::CONFIG_FILE_NAME;

    #[test]
    fn config_file_name_matches_cli_contract() {
        assert_eq!(CONFIG_FILE_NAME, "cross-kit.toml");
    }
}
