struct FirstViewModel;
struct SecondViewModel;

impl cross_kit::CkVmMetadata for FirstViewModel {
    fn ck_vm_metadata() -> &'static str {
        r#"{"name":"FirstViewModel"}"#
    }
}

impl cross_kit::CkVmMetadata for SecondViewModel {
    fn ck_vm_metadata() -> &'static str {
        r#"{"name":"SecondViewModel"}"#
    }
}

cross_kit::metadata_main!(FirstViewModel, SecondViewModel,);
