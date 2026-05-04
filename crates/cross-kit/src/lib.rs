//! Runtime entry point for Rust SDKs that integrate with Cross-Kit.
//!
//! Rust SDK crates should depend on this crate instead of depending on
//! Cross-Kit internal crates directly.

pub use cross_kit_core as core;
pub use cross_kit_core::{
    ArgMetadata, FactoryMetadata, MetadataValidationError, MethodMetadata, ObserverMetadata,
    VM_METADATA_SCHEMA_VERSION, VmMetadata, VmMode,
};
pub use cross_kit_macros::ck_vm_bridge as vm_bridge;

/// Metadata emitted by Cross-Kit VM bridge macros.
///
/// Generated metadata is consumed by the Cross-Kit CLI, code generators, and
/// platform packagers. Runtime SDK crates usually only need this trait so their
/// metadata binary can collect VM descriptions.
pub trait CkVmMetadata {
    fn ck_vm_metadata() -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::CkVmMetadata;

    struct ManualMetadata;

    impl CkVmMetadata for ManualMetadata {
        fn ck_vm_metadata() -> &'static str {
            r#"{"name":"manual"}"#
        }
    }

    #[test]
    fn metadata_trait_is_available_to_runtime_users() {
        let metadata: serde_json::Value =
            serde_json::from_str(ManualMetadata::ck_vm_metadata()).unwrap();
        assert_eq!(metadata["name"], "manual");
    }
}
