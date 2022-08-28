mod commands;
mod db;
mod logging;
mod parameters;
mod settings_db;
mod utils;
mod webhook;

use teloxide::prelude::*;

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

    let bot = Bot::from_env().auto_send();

    let message_clean_periodicity = parameters.message_clean_periodicity;
    let clean_databases_factory = pool_factory.clone();
    let _ = tokio::spawn(async move {
        let mut interval = tokio::time::interval(message_clean_periodicity);
        loop {
            interval.tick().await;
            clean_databases(clean_databases_factory.clone()).await;
        }
    });

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<commands::Command>()
                .endpoint(commands::command_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.forward_from_message_id().is_some()).endpoint(
                |msg: Message,
                 bot: AutoSend<Bot>,
                 pool_factory: std::sync::Arc<
                    tokio::sync::Mutex<db::SqliteDatabasePoolFactory>,
                >,
                 settings_db: std::sync::Arc<tokio::sync::Mutex<settings_db::SettingsDb>>| async move {
                    process_forward_message(pool_factory.clone(), settings_db.clone(), msg, bot)
                        .await?;
                    anyhow::Result::Ok(())
                },
            ),
        );

    if !parameters.is_webhook_mode_enabled {
        log::info!("Webhook deleted");
        bot.delete_webhook().await.expect("Cannot delete a webhook");
    }

    let mut bot_dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![
            pool_factory,
            settings_db,
            parameters.owner_id
        ])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .build();

    if parameters.is_webhook_mode_enabled {
        log::info!("Webhook mode activated");
        let rx = webhook::webhook(bot);
        bot_dispatcher
            .setup_ctrlc_handler()
            .dispatch_with_listener(
                rx.await,
                LoggingErrorHandler::with_custom_text("An error from the update listener"),
            )
            .await;
    } else {
        log::info!("Long polling mode activated");
        bot_dispatcher.setup_ctrlc_handler().dispatch().await;
    }
}

async fn clean_databases(
    pool_factory: std::sync::Arc<tokio::sync::Mutex<db::SqliteDatabasePoolFactory>>,
) {
    let chat_ids = pool_factory.lock().await.list_existing_chats();

    for chat_id in chat_ids {
        match pool_factory.lock().await.create(chat_id).await {
            Ok(chat) => match chat.clean_old_messages().await {
                Ok(_) => log::debug!("Chat with id={} cleaned successfully", chat_id),
                Err(e) => log::warn!("Error during chat with id={} cleaning: {}", chat_id, e),
            },
            Err(e) => log::warn!("Cannot open a chat database: {}", e),
        }
    }
}

async fn process_forward_message(
    pool_factory: std::sync::Arc<tokio::sync::Mutex<db::SqliteDatabasePoolFactory>>,
    settings_db: std::sync::Arc<tokio::sync::Mutex<settings_db::SettingsDb>>,
    msg: Message,
    bot: AutoSend<Bot>,
) -> anyhow::Result<()> {
    log::debug!("Start processing the message with a forward received");

    let mut pool_factory = pool_factory.lock().await;
    match pool_factory.create(msg.chat.id.0).await {
        Ok(client) => {
            let forwarded_message_id = msg
                .forward_from_message_id()
                .ok_or_else(|| anyhow!("Cannot find a forwarded message"))?;
            let sender_id = msg.from()
                .ok_or_else(|| anyhow!("Cannot find a message sender"))?.id.0 as i64;
            match client.check_forward_message(&forwarded_message_id, &sender_id).await {
                Ok(val) => {
                    if val {
                        utils::send_slowpoke(msg, bot, settings_db).await?;
                    } else if let Err(e) = client.add_forwarded_message(&forwarded_message_id, &sender_id).await
                    {
                        log::warn!("Cannot add a message to the database: {:?}", e);
                    }
                }
                Err(e) => log::warn!("Database error: {:?}", e),
            }
        }
        Err(e) => log::warn!("Cannot create a db client: {}", e),
    }

    anyhow::Result::Ok(())
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
