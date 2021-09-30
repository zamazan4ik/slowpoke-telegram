use crate::settings_db;
use crate::utils;
use teloxide::{prelude::*, utils::command::BotCommand};

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "display info about bot.")]
    About,
    #[command(description = "show help")]
    Help,
    #[command(description = "set reply image")]
    SetImage,
}

pub async fn command_answer(
    cx: &UpdateWithCx<Bot, Message>,
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
            cx.reply_to(ABOUT_TEXT).send().await?;
        }
        Command::Help => {
            if utils::is_sender_an_owner(&cx.update.from(), owner_id) {
                cx.reply_to(format!("{} {}", HELP_TEXT, HELP_TEXT_FOR_ADMIN))
                    .send()
                    .await?;
            } else {
                cx.reply_to(HELP_TEXT).send().await?;
            }
        }
        Command::SetImage => {
            if utils::is_sender_an_owner(&cx.update.from(), owner_id) {
                if let Some(reply_message) = cx.update.reply_to_message() {
                    if let Some(photo) = reply_message.photo() {
                        let first_photo = photo
                            .first()
                            .ok_or(anyhow!("Cannot extract a first photo from the reply"))?;

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
                        cx.reply_to(MISSED_PHOTO_IN_MESSAGE).send().await?;
                    }
                } else {
                    static MISSED_REPLY_MESSAGE: &str = "Чтобы установить изображение, Вам необходимо ответить на сообщение с требуемым изображнием";
                    cx.reply_to(MISSED_REPLY_MESSAGE).send().await?;
                }
            } else {
                cx.reply_to(PERMISSION_DENIED).send().await?;
            }
        }
    };

    Ok(())
}
