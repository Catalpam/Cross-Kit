use crate::cache::word_info::WordType;
use crate::result::{SDKError, SDKResult};
use crate::server::api::song as song_api;
use crate::server::proto::api::GetSongRecommendResponse;
use crate::server::proto::song::{Lyric, LyricElement, Song};
use crate::storage::song::{SongRecommendationStorage, SongStorage};
use crate::utils::nlp::spot_ruby::{PronouncedString, spot};
use chrono::format::StrftimeItems;
use rand::Rng;
use sqlx::SqlitePool;
use wana_kana::ConvertJapanese;

#[derive(Clone, uniffi::Record)]
pub struct CLSongBrief {
    pub id: i32,
    pub title: String,
    pub cover_image_url: String,
}

#[derive(Clone, uniffi::Record)]
pub struct CLFeedRecommands {
    pub recommand_id: i32,
    pub song_briefs: Vec<CLSongBrief>,
}

impl GetSongRecommendResponse {
    fn to_cl(self) -> CLFeedRecommands {
        CLFeedRecommands {
            recommand_id: self.recommended_id,
            song_briefs: self
                .briefs
                .into_iter()
                .map(|b| CLSongBrief { id: b.id, title: b.title, cover_image_url: "".to_string() })
                .collect(),
        }
    }
}

#[derive(Clone, uniffi::Record)]
pub struct CLSongInfo {
    pub id: i32,
    pub title: String,
    pub lyrics: Vec<CLSongLyric>,
}

impl Song {
    fn to_cl(self) -> CLSongInfo {
        CLSongInfo {
            id: self.id,
            title: self.title,
            lyrics: self.lyrics.into_iter().map(|l| l.to_cl()).collect(),
        }
    }
}

#[derive(Clone, uniffi::Record)]
pub struct CLSongLyric {
    pub id: i32,
    pub text: String,
    pub zh_cn: String,
    pub elements: Vec<CLSongLyricElement>,
}

impl Lyric {
    fn to_cl(self) -> CLSongLyric {
        CLSongLyric {
            id: rand::rng().random(),
            text: self.text,
            zh_cn: self.zh_translation,
            elements: self.elements.into_iter().map(|e| e.to_cl()).collect(),
        }
    }
}

#[derive(Clone, uniffi::Record)]
pub struct CLSongLyricElement {
    pub id: i32,
    pub surface: String,
    pub word_type: super::word_info::WordType,
    pub pronounced_string: Vec<PronouncedString>,
    pub ruby: String,
}

impl LyricElement {
    fn to_cl(self) -> CLSongLyricElement {
        let pronounced_string = spot(&self.surface.as_str(), &self.reading_form.as_str());
        CLSongLyricElement {
            id: rand::rng().random(),
            surface: self.surface,
            word_type: WordType::Adjective,
            pronounced_string: pronounced_string,
            ruby: self.reading_form.to_romaji(),
        }
    }
}

pub async fn get_song_by_id(pool: &SqlitePool, song_id: i32) -> SDKResult<CLSongInfo> {
    let storage = SongStorage::new(pool);

    // 先查找本地缓存是否存在
    if let Some(song) = storage.fetch(song_id).await? {
        return Ok(song.to_cl());
    }

    // 如果不存在则请求服务端获取
    let response = song_api::get_song_by_id(song_id).await?;
    let song = response.song.ok_or(SDKError::EmptyResponse)?;
    storage.upsert(&song).await?;

    let song = storage.fetch(song_id).await?.ok_or(SDKError::EmptyResponse)?;

    Ok(song.to_cl())
}

pub async fn get_feed_recommend_songs(
    pool: &SqlitePool,
    last_recommended_id: Option<i32>,
) -> SDKResult<CLFeedRecommands> {
    let storage = SongRecommendationStorage::new(pool);

    if let Some(last_id) = last_recommended_id {
        if let Some(response) = storage.fetch(last_id).await? {
            return Ok(response.to_cl());
        }
    }
    let last_id = last_recommended_id.unwrap_or(0);

    // 没有 id 或本地没查到，就去请求
    let response = song_api::get_song_recommend(last_recommended_id.unwrap_or(0)).await?;
    storage.upsert(last_id, &response).await?;

    storage.fetch(last_id).await?.ok_or(SDKError::EmptyResponse)?;
    Ok(response.to_cl())
}

pub async fn search_song(pool: &SqlitePool, keyword: &str) -> SDKResult<Vec<CLSongBrief>> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(SDKError::Other { message: "search keyword must not be empty".to_string() });
    }

    // Currently search results are not cached locally because they are lightweight and
    // depend heavily on the latest server-side ranking logic.
    let response = song_api::search_song(keyword.to_string()).await?;

    let briefs = response
        .briefs
        .into_iter()
        .map(|brief| CLSongBrief {
            id: brief.id,
            title: brief.title,
            cover_image_url: "".to_string(),
        })
        .collect();

    Ok(briefs)
}
