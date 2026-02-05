use crate::impl_mapping;
use crate::result::SDKResult;
use crate::server::mapping::common::{FromCommonResponse, ToCommonRequest, send_request};
use crate::server::proto::api::common_request::Request;
use crate::server::proto::api::common_response::Response;
use crate::server::proto::api::{LoginRequest, LoginResponse};

impl_mapping!(LoginRequest, LoginResponse, Request::Login, Response::Login);

pub async fn login(
    username: impl Into<String>,
    password: impl Into<String>,
) -> SDKResult<LoginResponse> {
    let req_body = LoginRequest { username: username.into(), password: password.into() };
    send_request(req_body).await
}
