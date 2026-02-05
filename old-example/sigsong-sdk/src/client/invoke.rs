use crate::client::client_ability::InvokeFFI;
use crate::result::{SDKError, SDKResult};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use uniffi::deps::anyhow::Context;

#[derive(uniffi::Object)]
#[derive(Debug)]
pub struct InvokeManager {
    pub rt: Runtime,
    pub pool: SqlitePool,
    pub invoke: Arc<dyn InvokeFFI>,
}

#[uniffi::export]
impl InvokeManager {
    #[uniffi::constructor]
    fn new(invoke: Arc<dyn InvokeFFI>) -> SDKResult<Arc<Self>> {
        let rt = Runtime::new().map_err(SDKError::from)?;

        let document_root = invoke.get_document_path().map_err(SDKError::from)?;

        let data_dir = Path::new(&document_root).join("data");
        println!("🚀rust_log: data directory {}", data_dir.display());

        ensure_folder(&data_dir)?;
        let pool = connect_to_db(&rt, &data_dir)?;

        let manager: Arc<InvokeManager> = Arc::new(Self { invoke, pool, rt });
        println!("🚀rust_log: 初始化EndInvokeManager成功");
        Ok(manager)
    }
}

fn connect_to_db(rt: &Runtime, data_dir: &Path) -> SDKResult<SqlitePool> {
    let db_path = data_dir.join("sqlx.db");
    let conn_str = format!("{}?mode=rwc", db_path.display());
    println!("🚀rust_log: sqlite path为 {}", conn_str);

    let options = SqliteConnectOptions::from_str(&conn_str)
        .map_err(|err| SDKError::database("parse sqlite connection options", err))?
        .pragma("key", "the_password123")
        .pragma("foreign_keys", "ON");

    println!("🚀rust_log: 数据库Option初始化完成");
    let pool = rt.block_on(async move {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|err| SDKError::database("connect to sqlite", err))?;
        println!("🚀rust_log: 数据库链接完成");
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|err| SDKError::database("run migrations", err.into()))?;
        println!("🚀rust_log: 数据库migrate完成");
        Ok::<_, SDKError>(pool)
    })?;

    Ok(pool)
}

fn ensure_folder(folder_path: &Path) -> SDKResult<()> {
    fs::create_dir_all(folder_path)
        .with_context(|| format!("unable to create folder {}", folder_path.display()))
        .map_err(SDKError::from)
}
