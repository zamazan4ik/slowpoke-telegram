use teloxide::prelude::*;

pub fn is_sender_an_owner(from: &Option<&teloxide::types::User>, owner_id: u64) -> bool {
    if let Some(user) = from {
        user.id.0 == owner_id
    } else {
        false
    }
}

pub async fn send_slowpoke(
    msg: Message,
    bot: AutoSend<Bot>,
    settings_db: std::sync::Arc<tokio::sync::Mutex<crate::settings_db::SettingsDb>>,
) -> anyhow::Result<()> {
    match settings_db.lock().await.get_setting("image_file_id") {
        Ok(value) => {
            log::debug!("Image file id: {}", value);

            if let Err(e) = bot
                .send_photo(msg.chat.id, teloxide::types::InputFile::file_id(value))
                .reply_to_message_id(msg.id)
                .await
            {
                log::warn!("Cannot send a response: {:?}", e);
            }
        }
        Err(e) => {
            log::warn!("Cannot get an image from database: {:?}", e);
            static MISSED_SLOWPOKE_IN_DATABASE: &str = "Слоупоки закончились :(.";
            bot.send_message(msg.chat.id, MISSED_SLOWPOKE_IN_DATABASE)
                .reply_to_message_id(msg.id)
                .await?;
        }
    }

    Ok(())
}
