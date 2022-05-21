use crate::settings_db;
use crate::utils;
use teloxide::{prelude2::*, utils::command::BotCommand};
//use teloxide_core::types::BotCommand;

#[derive(Clone, teloxide::utils::command::BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "display info about bot.")]
    About,
    #[command(description = "show help")]
    Help,
    #[command(description = "set reply image")]
    SetImage,
}

pub async fn command_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    command: Command,
    owner_id: i64,
    settings_db: std::sync::Arc<tokio::sync::Mutex<settings_db::SettingsDb>>,
) -> anyhow::Result<()> {
    static ABOUT_TEXT: &str = "По всем замечаниям или предложениям обращаться сюда:\
        https://github.com/ZaMaZaN4iK/slowpoke-telegram . Спасибо!";

    static HELP_TEXT: &str = "Бот просто определяет, являетесь ли вы Слоупоком или нет :)";
    static HELP_TEXT_FOR_ADMIN: &str =
        "Чтобы установить изображение для бота, ответьте командой /setimage на сообщение с изображением.";
    static PERMISSION_DENIED: &str = "У вас недостаточно прав для выполнения данной операции!";

    match command {
        Command::About => {
            bot.send_message(msg.chat.id, ABOUT_TEXT)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Command::Help => {
            let help_text = if utils::is_sender_an_owner(&msg.from(), owner_id) {
                format!("{} {}", HELP_TEXT, HELP_TEXT_FOR_ADMIN)
            } else {
                HELP_TEXT.to_string()
            };
            bot.send_message(msg.chat.id, help_text)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Command::SetImage => {
            if utils::is_sender_an_owner(&msg.from(), owner_id) {
                if let Some(reply_message) = msg.reply_to_message() {
                    if let Some(photo) = reply_message.photo() {
                        let first_photo = photo.first().ok_or_else(|| {
                            anyhow!("Cannot extract a first photo from the reply")
                        })?;

                        let image_set_result = settings_db
                            .lock()
                            .await
                            .add_setting("image_file_id", first_photo.file_id.as_str());

                        match image_set_result {
                            Ok(_) => log::info!("Image was updated successfully"),
                            Err(e) => log::info!("Image was not updated successfully: {:?}", e),
                        }
                    } else {
                        static MISSED_PHOTO_IN_MESSAGE: &str =
                            "Не могу обнаружить фото в цитируемом сообщении.";
                        bot.send_message(msg.chat.id, MISSED_PHOTO_IN_MESSAGE)
                            .reply_to_message_id(msg.id)
                            .await?;
                    }
                } else {
                    static MISSED_REPLY_MESSAGE: &str = "Чтобы установить изображение, Вам необходимо ответить на сообщение с требуемым изображнием";
                    bot.send_message(msg.chat.id, MISSED_REPLY_MESSAGE)
                        .reply_to_message_id(msg.id)
                        .await?;
                }
            } else {
                bot.send_message(msg.chat.id, PERMISSION_DENIED)
                    .reply_to_message_id(msg.id)
                    .await?;
            }
        }
    };

    Ok(())
}
