use teloxide::{prelude::*, utils::command::BotCommand};

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "display info about bot.")]
    About,
}

pub async fn command_answer(cx: &UpdateWithCx<Message>, command: Command) -> ResponseResult<()> {
    static ABOUT_TEXT: &str = "По всем замечаниям или предложениям обращаться сюда:\
        https://github.com/ZaMaZaN4iK/slowpoke-telegram . Спасибо!";

    match command {
        Command::About => cx.reply_to(ABOUT_TEXT).send().await?,
    };

    Ok(())
}
