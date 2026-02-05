use crate::result::{SDKError, SDKResult};
use crate::server::proto::api::GetSongRecommendResponse;
use crate::server::proto::song::Song;
use prost::Message;
use sqlx::{Row, SqlitePool};

pub struct SongStorage<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SongStorage<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn fetch(&self, song_id: i32) -> SDKResult<Option<Song>> {
        let record = sqlx::query("SELECT payload FROM cached_songs WHERE id = ?")
            .bind(song_id)
            .fetch_optional(self.pool)
            .await
            .map_err(|err| SDKError::database(format!("load cached song {song_id}"), err))?;

        if let Some(row) = record {
            let payload: Vec<u8> = row
                .try_get("payload")
                .map_err(|err| SDKError::database("read cached song payload", err))?;
            let song =
                Song::decode(payload.as_slice()).map_err(|err| SDKError::decode("Song", err))?;
            Ok(Some(song))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert(&self, song: &Song) -> SDKResult<()> {
        let payload = song.encode_to_vec();
        sqlx::query(
            "INSERT INTO cached_songs (id, payload, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(id) DO UPDATE SET payload = excluded.payload, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(song.id)
        .bind(payload)
        .execute(self.pool)
        .await
        .map_err(|err| SDKError::database(format!("upsert cached song {}", song.id), err))?;

        Ok(())
    }
}

pub struct SongRecommendationStorage<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SongRecommendationStorage<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    fn cache_key(last_recommended_id: i32) -> String {
        format!("last_id:{last_recommended_id}")
    }

    pub async fn fetch(
        &self,
        last_recommended_id: i32,
    ) -> SDKResult<Option<GetSongRecommendResponse>> {
        let key = Self::cache_key(last_recommended_id);
        let record =
            sqlx::query("SELECT payload FROM cached_song_recommendations WHERE cache_key = ?")
                .bind(&key)
                .fetch_optional(self.pool)
                .await
                .map_err(|err| {
                    SDKError::database(format!("load cached recommendations for {key}"), err)
                })?;

        if let Some(row) = record {
            let payload: Vec<u8> = row
                .try_get("payload")
                .map_err(|err| SDKError::database("read cached recommendation payload", err))?;
            let response = GetSongRecommendResponse::decode(payload.as_slice())
                .map_err(|err| SDKError::decode("GetSongRecommendResponse", err))?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    pub async fn upsert(
        &self,
        last_recommended_id: i32,
        response: &GetSongRecommendResponse,
    ) -> SDKResult<()> {
        let key = Self::cache_key(last_recommended_id);
        let payload = response.encode_to_vec();

        sqlx::query(
            "INSERT INTO cached_song_recommendations (cache_key, payload, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)\n             ON CONFLICT(cache_key) DO UPDATE SET payload = excluded.payload, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&key)
        .bind(payload)
        .execute(self.pool)
        .await
        .map_err(|err| SDKError::database(format!("upsert cached recommendations for {key}"), err))?;

        Ok(())
    }
}
