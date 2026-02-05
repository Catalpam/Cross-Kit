use crate::impl_mapping;
use crate::result::SDKResult;
use crate::server::mapping::common::send_request;
use crate::server::mapping::common::{FromCommonResponse, ToCommonRequest};
use crate::server::proto::api::common_request::Request;
use crate::server::proto::api::common_response::Response;
use crate::server::proto::api::{
    GetSongByIdRequest, GetSongByIdResponse, GetSongRecommendRequest, GetSongRecommendResponse,
    SearchSongRequest, SearchSongResponse,
};

impl_mapping!(GetSongByIdRequest, GetSongByIdResponse, Request::GetSongById, Response::GetSongById);
pub async fn get_song_by_id(song_id: i32) -> SDKResult<GetSongByIdResponse> {
    println!("{:?}", "歌词开始请求".to_string());
    let req_body = GetSongByIdRequest { id: song_id };
    send_request(req_body).await
}

impl_mapping!(
    GetSongRecommendRequest,
    GetSongRecommendResponse,
    Request::GetSongRecommend,
    Response::GetSongRecommend
);
pub async fn get_song_recommend(last_recommended_id: i32) -> SDKResult<GetSongRecommendResponse> {
    println!("{:?}", "歌词开始请求".to_string());
    let req_body = GetSongRecommendRequest { last_recommended_id };
    send_request(req_body).await
}

impl_mapping!(SearchSongRequest, SearchSongResponse, Request::SearchSong, Response::SearchSong);
pub async fn search_song(keyword: impl Into<String>) -> SDKResult<SearchSongResponse> {
    let keyword = keyword.into();
    println!("search song request keyword length {:?}", keyword.len());
    let req_body = SearchSongRequest { keyword };
    send_request(req_body).await
}
