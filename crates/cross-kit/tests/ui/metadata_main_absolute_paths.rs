pub struct RootViewModel;
pub struct ChildViewModel;

impl cross_kit::CkVmMetadata for RootViewModel {
    fn ck_vm_metadata() -> &'static str {
        r#"{"name":"RootViewModel"}"#
    }
}

impl cross_kit::CkVmMetadata for ChildViewModel {
    fn ck_vm_metadata() -> &'static str {
        r#"{"name":"ChildViewModel"}"#
    }
}

cross_kit::metadata_main!(crate::RootViewModel, crate::ChildViewModel);
