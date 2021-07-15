#[derive(serde::Serialize, serde::Deserialize)]
struct AnimationSettings {
    file_id: std::string::String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ImageSettings {
    file_id: std::string::String,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum ResponseMediaType {
    Animation(AnimationSettings),
    Image(ImageSettings),
}

pub struct SettingsDb {
    database_pool: sqlx::SqlitePool,
}

#[derive(sqlx::FromRow)]
struct Value(std::string::String);

impl SettingsDb {
    pub async fn new(
        db_location: &std::path::Path,
        max_database_connections: u32,
    ) -> anyhow::Result<Self> {
        let new_db_path = db_location.join("settings.db");

        let connection_string = format!(
            "{}",
            new_db_path
                .to_str()
                .ok_or(anyhow!("Cannot convert a database path to a string"))?
        );

        let connection_options = sqlx::sqlite::SqliteConnectOptions::default()
            .create_if_missing(true)
            .filename(connection_string);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(max_database_connections)
            .connect_with(connection_options)
            .await?;

        Self::init_new_db(pool.clone()).await?;

        Ok(Self {
            database_pool: pool,
        })
    }

    pub async fn get_setting(&self, name: &str) -> anyhow::Result<Value, sqlx::Error> {
        sqlx::query_as::<_, Value>("SELECT value FROM settings WHERE key = ?")
            .bind(name)
            .fetch_one(&self.database_pool)
            .await
    }

    pub async fn add_setting(
        &mut self,
        key: &str,
        value: &str,
    ) -> anyhow::Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query("INSERT INTO statistic (key,value) VALUES(?,?)")
            .bind(key)
            .bind(value)
            .execute(&self.database_pool)
            .await
    }

    async fn init_new_db(
        db: sqlx::SqlitePool,
    ) -> anyhow::Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (
                key INTEGER PRIMARY KEY NOT NULL,
                value TEXT NOT NULL);",
        )
        .execute(&db)
        .await
    }
}
