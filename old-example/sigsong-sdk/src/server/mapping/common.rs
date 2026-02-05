use crate::result::{SDKError, SDKResult};
use crate::server::base_url;
use crate::server::proto::api::CommonRequest;
use crate::server::proto::api::CommonResponse;
use crate::server::proto::api::common_request::Request;
use crate::server::proto::api::common_response::Response;
use crate::server::proto::api::common_response::Response::Error;
use crate::utils::compression::{compress, decompress_bytes};
// use anyhow::Context;
use bytes::Bytes;
use prost::Message;

fn get_common_request(req_body: Request) -> CommonRequest {
    CommonRequest { rid: "123456".to_string(), request: Some(req_body) }
}

pub async fn send_request<Q, R>(req_body: Q) -> SDKResult<R>
where
    Q: ToCommonRequest,
    R: FromCommonResponse,
{
    println!("{:?}", "开始请求".to_string());
    let req = get_common_request(req_body.into_common_request());
    let client = reqwest::Client::new();
    let compressed_bytes = compress(req.encode_to_vec())
        // .with_context(|| "failed to compress request payload".to_string())
        .map_err(SDKError::from)?;
    println!("请求体压缩后字节数 {:?}", compressed_bytes.len());
    let response_bytes = client
        .post(base_url())
        .body(compressed_bytes)
        .send()
        .await
        .map_err(|err| SDKError::network("sending request to backend", err))?
        .bytes()
        .await
        .map_err(|err| SDKError::network("reading response bytes", err))?;
    println!("返回体压缩时字节数： {:?}", response_bytes.len());
    let response_bytes = decompress_bytes(response_bytes)
        // .with_context(|| "failed to decompress response payload".to_string())
        .map_err(SDKError::from)?;
    println!("返回体解压后字节数： {:?}", response_bytes.len());
    let full_response = CommonResponse::decode(Bytes::from(response_bytes))
        .map_err(|err| SDKError::decode("CommonResponse", err))?;
    let response = full_response.response.ok_or(SDKError::EmptyResponse)?;
    if let Error(error) = response {
        Err(error.to_sdk_error())
    } else {
        R::from_common_response(response).ok_or(SDKError::ResponseTypeMismatch)
    }
}

pub trait ToCommonRequest {
    fn into_common_request(self) -> Request;
}
pub trait FromCommonResponse {
    fn from_common_response(response: Response) -> Option<Self>
    where
        Self: Sized;
}
