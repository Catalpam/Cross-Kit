use crate::cache;
use crate::cache::song::{CLFeedRecommands, CLSongBrief, CLSongInfo};
use crate::client::invoke::InvokeManager;
use crate::result::SDKResult;
use crate::server::proto::api::GetSongRecommendResponse;
use crate::server::proto::song::Song;

#[uniffi::export]
impl InvokeManager {
    pub async fn get_song_by_id(&self, id: i32) -> SDKResult<CLSongInfo> {
        self.rt.block_on(cache::song::get_song_by_id(&self.pool, id))
    }

    pub async fn get_feed_recommend_songs(
        &self,
        last_recommended_id: Option<i32>,
    ) -> SDKResult<CLFeedRecommands> {
        self.rt.block_on(cache::song::get_feed_recommend_songs(&self.pool, last_recommended_id))
    }

    pub async fn search_song(&self, keyword: String) -> SDKResult<Vec<CLSongBrief>> {
        self.rt.block_on(cache::song::search_song(&self.pool, keyword.as_str()))
    }
}
