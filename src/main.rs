mod commands;
mod detection;
mod logging;
mod utils;
mod webhook;

use teloxide::{prelude::*, utils::command::BotCommand};

use sqlx::SqlitePool;

use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    logging::init_logger();
    log::info!("Starting CodeDetector bot");

    let is_webhook_mode_enabled = env::var("WEBHOOK_MODE")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .expect(
            "Cannot convert WEBHOOK_MODE to bool. Applicable values are only \"true\" or \"false\"",
        );
    let database_max_connections_count: u32 = env::var("MAX_DB_CONNECTIONS")
        .unwrap_or("5".to_string())
        .parse()
        .expect("MAX_DB_CONNECTIONS value has to be an unsigned integer");

    let pool = SqlitePoolOptions::new()
        .max_connections(database_max_connections_count)
        .connect("postgres://postgres:password@localhost/test").await?;

    let bot = Bot::from_env();

    let bot_dispatcher = Dispatcher::new(bot.clone())
        .messages_handler(move |rx: DispatcherHandlerRx<Message>| {
            rx.for_each(move |message| {
                async move {
                    let message_text = match message.update.text() {
                        Some(x) => x,
                        None => return,
                    };

                    // Handle commands. If command cannot be parsed - continue processing
                    match commands::Command::parse(message_text, "SlowpokeBot") {
                        Ok(command) => {
                            commands::command_answer(&message, command)
                                .await
                                .log_on_error()
                                .await;
                            return;
                        }
                        Err(_) => (),
                    };

                    // For now we check whole message for being an URL.
                    // We are not trying to find sub-URLs in a message, since it can lead to too high
                    // false positives rate
                    if detection::is_url(message_text) {
                        // TODO: Check in a corresponding database and send slowpoke message, if such
                        // link was earlier :)

                    }
                }
            })
        });

    if is_webhook_mode_enabled {
        log::info!("Webhook mode activated");
        let rx = webhook::webhook(bot);
        bot_dispatcher
            .dispatch_with_listener(
                rx.await,
                LoggingErrorHandler::with_custom_text("An error from the update listener"),
            )
            .await;
    } else {
        log::info!("Long polling mode activated");
        bot.delete_webhook()
            .send()
            .await
            .expect("Cannot delete a webhook");
        bot_dispatcher.dispatch().await;
    }
}
