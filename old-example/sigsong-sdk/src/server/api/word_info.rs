use super::super::proto::api::GetWordInfoRequest;
use crate::impl_mapping;
use crate::result::SDKResult;
use crate::server::mapping::common::send_request;
use crate::server::mapping::common::{FromCommonResponse, ToCommonRequest};
use crate::server::proto::api::GetWordInfoResponse;
use crate::server::proto::api::common_request::Request;
use crate::server::proto::api::common_response::Response;

impl_mapping!(GetWordInfoRequest, GetWordInfoResponse, Request::GetWordInfo, Response::GetWordInfo);

pub async fn get_word_info(word: String) -> SDKResult<GetWordInfoResponse> {
    let mut req_body = GetWordInfoRequest::default();
    req_body.word = word;
    send_request(req_body).await
}
