use sqlx::sqlite::SqliteQueryResult;
use sqlx::Error;

pub struct SqliteDatabasePoolFactory {
    db_root_path: std::path::PathBuf,
    max_connections_per_db: u32,
    database_pools: std::collections::HashMap<i64, sqlx::SqlitePool>,
    client_pool: std::collections::HashMap<i64, std::sync::Arc<ChatDatabase>>,
}

pub struct ChatDatabase {
    database_pool: sqlx::SqlitePool,
}

impl ChatDatabase {
    pub fn new(database_pool: sqlx::SqlitePool) -> Self {
        Self { database_pool }
    }

    pub async fn check_forward_message(&self, forward_message_id: &i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "SELECT message_id FROM forwarded_message WHERE message_id = ? AND timestamp >= date('now', '-1 day')",
        )
        .bind(forward_message_id)
        .fetch_optional(&self.database_pool).await?;

        Ok(result.is_some())
    }

    pub async fn add_forwarded_message(
        &self,
        forward_message_id: &i32,
    ) -> Result<SqliteQueryResult, Error> {
        sqlx::query("INSERT INTO forwarded_message (message_id) VALUES(?)")
            .bind(forward_message_id)
            .execute(&self.database_pool)
            .await
    }

    pub async fn clean_old_messages(&self) -> Result<SqliteQueryResult, Error> {
        sqlx::query("DELETE FROM forwarded_message WHERE timestamp < date('now', '-3 day');")
            .execute(&self.database_pool)
            .await
    }
}

impl SqliteDatabasePoolFactory {
    pub fn new(db_root_path: std::path::PathBuf, max_connections_per_db: u32) -> Self {
        Self {
            db_root_path,
            max_connections_per_db,
            database_pools: Default::default(),
            client_pool: Default::default(),
        }
    }

    pub fn list_existing_chats(&self) -> std::vec::Vec<i64> {
        let chat_paths = std::fs::read_dir(self.db_root_path.as_path()).unwrap();

        let mut chats = Vec::new();

        for chat_path in chat_paths {
            match chat_path {
                Ok(chat_path) => {
                    // We want to skip all db files, except .db extensions. We need to do it since SQLite spawns temporal files
                    if let Some(extension) = chat_path.path().extension() {
                        if extension != "db" {
                            continue;
                        }
                    } else {
                        // Files with no extension also should be skipped
                        continue;
                    }
                    if let Some(chat_name) = chat_path.path().file_stem() {
                        if let Some(chat_name) = chat_name.to_str() {
                            match chat_name.parse::<i64>() {
                                Ok(chat_id) => chats.push(chat_id),
                                Err(e) => log::warn!("Cannot parse chat_name into i64: {}", e),
                            };
                        } else {
                            log::warn!("Cannot convert chat_name to String")
                        }
                    } else {
                        log::warn!("Cannot extract file stem")
                    }
                }
                Err(e) => log::warn!("Cannot read dir entry: {}", e),
            }
        }

        chats
    }

    pub async fn init_new_db(&self, db: sqlx::SqlitePool) -> Result<SqliteQueryResult, Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS forwarded_message (
                message_id INTEGER PRIMARY KEY NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP);",
        )
        .execute(&db)
        .await
    }

    pub async fn create(&mut self, chat_id: i64) -> anyhow::Result<std::sync::Arc<ChatDatabase>> {
        if let Some(client) = self.client_pool.get(&chat_id) {
            Ok(client.clone())
        } else {
            let new_db_path = self.db_root_path.join(format!("{}.db", chat_id));

            let connection_string = (new_db_path
                .to_str()
                .ok_or_else(|| anyhow!("Cannot convert a database path to a string"))?)
            .to_string();

            log::info!("{}", connection_string);

            let connection_options = sqlx::sqlite::SqliteConnectOptions::default()
                .create_if_missing(true)
                .filename(connection_string);

            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(self.max_connections_per_db)
                .connect_with(connection_options)
                .await?;

            self.init_new_db(pool.clone()).await?;

            self.database_pools.insert(chat_id, pool.clone());

            let new_client = std::sync::Arc::new(ChatDatabase::new(pool));
            self.client_pool.insert(chat_id, new_client.clone());

            Ok(new_client)
        }
    }
}
