use crate::client::invoke::InvokeManager;
use crate::result::SDKResult;
use std::fmt::Debug;

pub trait ClientAbility {
    fn document_path(&self) -> SDKResult<String>;
    fn show_toast(&self, toast_type: ToastType, message: String);
}

impl ClientAbility for InvokeManager {
    fn document_path(&self) -> SDKResult<String> {
        self.invoke.get_document_path()
    }

    fn show_toast(&self, toast_type: ToastType, message: String) {
        let invoke = self.invoke.clone();
        self.rt.spawn(async move {
            invoke.show_toast(toast_type, message);
        });
    }
}

#[uniffi::export(with_foreign)]
pub(crate) trait InvokeFFI: Send + Sync + Debug {
    fn get_document_path(&self) -> SDKResult<String>;
    fn show_toast(&self, toast_type: ToastType, message: String);
}

#[derive(uniffi::Enum, Debug)]
pub enum ToastType {
    Info,
    Success,
    Error,
}
