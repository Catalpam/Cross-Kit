use crate::cache::user::{self, CLUserInfo};
use crate::client::client_ability::{ClientAbility, ToastType};
use crate::client::invoke::InvokeManager;
use crate::result::{SDKError, SDKResult};

#[uniffi::export]
impl InvokeManager {
    pub async fn login(&self, username: String, password: String) -> SDKResult<CLUserInfo> {
        let username = username.trim().to_string();
        let password = password.trim().to_string();

        if username.is_empty() || password.is_empty() {
            return Err(SDKError::Other { message: "账号或密码不能为空".to_string() });
        }

        let pool = self.pool.clone();
        let info =
            self.rt.block_on(async move { user::login(&pool, &username, &password).await })?;

        self.show_toast(ToastType::Success, "登录成功".to_string());
        Ok(info)
    }

    pub async fn current_user(&self) -> SDKResult<Option<CLUserInfo>> {
        let pool = self.pool.clone();
        self.rt.block_on(async move { user::current_user(&pool).await })
    }

    pub async fn logout(&self) -> SDKResult<()> {
        let pool = self.pool.clone();
        self.rt.block_on(async move { user::logout(&pool).await })?;
        self.show_toast(ToastType::Info, "已退出登录".to_string());
        Ok(())
    }
}
