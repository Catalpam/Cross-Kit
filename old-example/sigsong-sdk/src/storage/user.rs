use crate::result::{SDKError, SDKResult};
use sqlx::{Row, SqlitePool};

#[derive(Clone, Debug)]
pub struct StoredUser {
    pub id: u32,
    pub name: String,
    pub avatar_key: String,
    pub token: String,
}

pub struct UserStorage<'a> {
    pool: &'a SqlitePool,
}

impl<'a> UserStorage<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, user: &StoredUser) -> SDKResult<()> {
        sqlx::query(
            "INSERT INTO current_user (id, name, avatar_key, token, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)\n             ON CONFLICT(id) DO UPDATE SET name = excluded.name, avatar_key = excluded.avatar_key, token = excluded.token, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(user.id as i64)
        .bind(&user.name)
        .bind(&user.avatar_key)
        .bind(&user.token)
        .execute(self.pool)
        .await
        .map_err(|err| SDKError::database("upsert current user", err))?;

        Ok(())
    }

    pub async fn fetch(&self) -> SDKResult<Option<StoredUser>> {
        let record = sqlx::query("SELECT id, name, avatar_key, token FROM current_user LIMIT 1")
            .fetch_optional(self.pool)
            .await
            .map_err(|err| SDKError::database("fetch current user", err))?;

        if let Some(row) = record {
            let id: i64 =
                row.try_get("id").map_err(|err| SDKError::database("read user id", err))?;
            let name: String =
                row.try_get("name").map_err(|err| SDKError::database("read user name", err))?;
            let avatar_key: String = row
                .try_get("avatar_key")
                .map_err(|err| SDKError::database("read avatar key", err))?;
            let token: String =
                row.try_get("token").map_err(|err| SDKError::database("read token", err))?;

            Ok(Some(StoredUser { id: id as u32, name, avatar_key, token }))
        } else {
            Ok(None)
        }
    }

    pub async fn clear(&self) -> SDKResult<()> {
        sqlx::query("DELETE FROM current_user")
            .execute(self.pool)
            .await
            .map_err(|err| SDKError::database("clear current user", err))?;
        Ok(())
    }
}
