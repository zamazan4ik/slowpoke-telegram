pub struct Parameters {
    pub bot_name: String,
    pub owner_id: u64,
    pub settings_database_path: std::path::PathBuf,
    pub chat_database_root_path: std::path::PathBuf,
    pub max_database_connections_count: u32,
    pub max_message_age: std::time::Duration,
    pub message_clean_periodicity: std::time::Duration,
    pub is_webhook_mode_enabled: bool,
}

impl Parameters {
    pub fn new() -> Self {
        let bot_name = std::env::var("BOT_NAME").expect("BOT_NAME env var is not specified");

        let owner_id: u64 = std::env::var("OWNER_ID")
            .expect("OWNER_ID env var is not specified")
            .parse()
            .expect("Cannot parse as u64");

        let settings_database_path: std::path::PathBuf = std::env::var("SETTINGS_DATABASE_PATH")
            .expect("SETTINGS_DATABASE_PATH is not specified")
            .parse()
            .expect("Cannot parse as a filepath");

        let chat_database_root_path: std::path::PathBuf = std::env::var("CHAT_DATABASE_PATH")
            .expect("CHAT_DATABASE_PATH is not specified")
            .parse()
            .expect("Cannot parse as a filepath");

        let max_database_connections_count: u32 = std::env::var("MAX_DB_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .expect("MAX_DB_CONNECTIONS value has to be an unsigned integer");

        let max_message_age = std::time::Duration::from_secs(
            std::env::var("MAX_MESSAGE_AGE_IN_SECONDS")
                .unwrap_or_else(|_| {
                    std::time::Duration::from_secs(3 * 24 * 60 * 60)
                        .as_secs()
                        .to_string()
                })
                .parse()
                .expect("Cannot parse provided time as seconds"),
        );

        let message_clean_periodicity = std::time::Duration::from_secs(
            std::env::var("MESSAGE_CLEAN_PERIODICITY_IN_SECONDS")
                .unwrap_or_else(|_| {
                    std::time::Duration::from_secs(24 * 60 * 60)
                        .as_secs()
                        .to_string()
                })
                .parse()
                .expect("Cannot parse provided time as seconds"),
        );

        let is_webhook_mode_enabled: bool = std::env::var("WEBHOOK_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .expect(
                "Cannot convert WEBHOOK_MODE to bool. Applicable values are only \"true\" or \"false\"",
            );

        Self {
            bot_name,
            owner_id,
            settings_database_path,
            chat_database_root_path,
            max_database_connections_count,
            max_message_age,
            message_clean_periodicity,
            is_webhook_mode_enabled,
        }
    }
}
