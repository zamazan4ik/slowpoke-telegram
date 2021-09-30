mod commands;
mod db;
mod detection;
mod logging;
mod parameters;
mod settings_db;
mod utils;
mod webhook;

use teloxide::{prelude::*, utils::command::BotCommand};

use teloxide::types::InputFile;

#[macro_use]
extern crate anyhow;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    logging::init_logger();
    log::info!("Starting slowpoke bot");

    let parameters = std::sync::Arc::new(parameters::Parameters::new());

    let settings_db = std::sync::Arc::new(tokio::sync::Mutex::new(
        settings_db::SettingsDb::new(parameters.settings_database_path.as_path())
            .expect("Cannot open settings database"),
    ));

    let pool_factory =
        std::sync::Arc::new(tokio::sync::Mutex::new(db::SqliteDatabasePoolFactory::new(
            parameters.chat_database_root_path.clone(),
            parameters.max_database_connections_count,
        )));

    let bot = Bot::from_env();
    let bot_parameters = parameters.clone();

    let mut bot_dispatcher = Dispatcher::new(bot.clone()).messages_handler(
        move |rx: DispatcherHandlerRx<Bot, Message>| {
            let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
            rx.for_each(move |message| {
                let bot_name = bot_parameters.bot_name.clone();
                let settings_db = settings_db.clone();
                let owner_id = bot_parameters.owner_id;
                let pool_factory = pool_factory.clone();
                async move {
                    if let Some(message_text) = message.update.text() {
                        // Handle commands. If command cannot be parsed - continue processing
                        match commands::Command::parse(message_text, bot_name) {
                            Ok(command) => {
                                commands::command_answer(&message, command, owner_id, settings_db)
                                    .await
                                    .log_on_error()
                                    .await;
                                return;
                            }
                            Err(_) => (),
                        };
                    }

                    log::info!("Handler is triggered");

                    // Check for forwarded messages
                    if let Some(forwarded_message_id) = message.update.forward_from_message_id() {
                        let mut pool_factory = pool_factory.lock().await;
                        match pool_factory.create(message.update.chat_id()).await {
                            Ok(client) => {
                                match client.check_forward_message(forwarded_message_id).await {
                                    Ok(val) => {
                                        if val {
                                            match settings_db
                                                .lock()
                                                .await
                                                .get_setting("image_file_id")
                                            {
                                                Ok(value) => {
                                                    log::info!("{}", value);

                                                    if let Err(e) = message
                                                        .answer_photo(InputFile::FileId(value))
                                                        .send()
                                                        .await
                                                    {
                                                        log::info!(
                                                            "Cannot send a response: {:?}",
                                                            e
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    log::info!("Cannot get a setting: {:?}", e)
                                                }
                                            }
                                        } else {
                                            if let Err(e) = client
                                                .add_forwarded_message(forwarded_message_id)
                                                .await
                                            {
                                                log::warn!(
                                                    "Cannot add a message to the database: {:?}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => log::warn!("Database error: {:?}", e),
                                }
                            }
                            Err(e) => log::warn!("Cannot create a db client: {}", e),
                        }

                        // 1) Check in a database. If exists - send a slowpoke and update timestamp
                        // 2) If doesn't exist - push to a database [primary_id; current_timestamp; forwarded_message_id]
                    }

                    // For now we check whole message for being an URL.
                    // We are not trying to find sub-URLs in a message, since it can lead to too high
                    // false positives rate
                    /*if detection::is_url(message_text) {
                        // TODO: Check in a corresponding database and send slowpoke message, if such
                        // link was earlier :)
                    }*/
                }
            })
        },
    );

    if parameters.is_webhook_mode_enabled {
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

// Message types for check
// 1) Images. For checks perceptual hash can be used.
// 2) Forwards from the same channels. forward flag + original message id from source channel/user can be used
// 3) Messages with links. Extract links from messages and store them
// 4) Forwarding the same content from different channels - ? With images we can use perceptual hash. Video - possibly first frame + perceptual hash

// Set time threshold for slowpoke bot - check only messages for a several days (configurable), since most duplicates appear at the same day.
// Also it will reduce false positive count

// Reply image configuration: after bot start owner fill set an image in private dialogue with bot. Bot will get file_id from it and save in persistent storage (yet another own database).
// If storage will be missed - bot will ask again about the image (or file_id, if image is still exists)
