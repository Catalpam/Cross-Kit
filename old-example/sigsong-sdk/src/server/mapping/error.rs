use crate::result::SDKError;
use crate::server::proto::api::CommonApiError;

impl CommonApiError {
    pub fn to_sdk_error(self) -> SDKError {
        SDKError::ServerError { reason: self.error, message: self.server_message }
    }
}
