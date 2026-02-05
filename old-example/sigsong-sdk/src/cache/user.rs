use crate::result::SDKResult;
use crate::server::api::passport;
use crate::storage::user::{StoredUser, UserStorage};
use sqlx::SqlitePool;

#[derive(Clone, uniffi::Record)]
pub struct CLUserInfo {
    pub id: u32,
    pub name: String,
    pub avatar_key: String,
    pub token: String,
}

impl From<StoredUser> for CLUserInfo {
    fn from(value: StoredUser) -> Self {
        Self { id: value.id, name: value.name, avatar_key: value.avatar_key, token: value.token }
    }
}

impl From<&CLUserInfo> for StoredUser {
    fn from(value: &CLUserInfo) -> Self {
        StoredUser {
            id: value.id,
            name: value.name.clone(),
            avatar_key: value.avatar_key.clone(),
            token: value.token.clone(),
        }
    }
}

pub async fn login(pool: &SqlitePool, username: &str, password: &str) -> SDKResult<CLUserInfo> {
    let response = passport::login(username.to_string(), password.to_string()).await?;
    let user = response.user.ok_or(crate::result::SDKError::EmptyResponse)?;

    let info = CLUserInfo {
        id: user.id,
        name: user.name,
        avatar_key: user.avatar_key,
        token: response.token,
    };

    let storage = UserStorage::new(pool);
    storage.upsert(&StoredUser::from(&info)).await?;

    Ok(info)
}

pub async fn current_user(pool: &SqlitePool) -> SDKResult<Option<CLUserInfo>> {
    let storage = UserStorage::new(pool);
    let user = storage.fetch().await?.map(CLUserInfo::from);
    Ok(user)
}

pub async fn logout(pool: &SqlitePool) -> SDKResult<()> {
    let storage = UserStorage::new(pool);
    storage.clear().await
}
