use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::anyhow;

use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::*,
    update_listeners::webhooks,
};
use teloxide::types::MessageId;
use tokio::sync::Mutex;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AskQuestion,
}

#[derive(Clone)]
struct ConfigParameters {
    admins_group_id: ChatId,
    public_channel_id: ChatId,
}


fn get_channel_id() -> ChatId {
    return ChatId(
        std::env::var("CHANNEL_ID")
            .expect("Unable to read CHANNEL_ID from ENV")
            .parse()
            .expect("Unable to parse CHANNEL_ID as i64"),
    );
}

struct DialogInfo {
    source_channel_id: ChatId,
    source_message_id: MessageId,
    source_in_channel_message_id: MessageId,
}

#[tokio::main]
async fn main() {
    match dotenvy::dotenv() {
        Err(err) => {
            log::info!("Fail to read .env file {:?}", err);
        }
        _ => {}
    };
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();
    let admin_channel_id = get_channel_id();
    let public_channel_id = ChatId(
        std::env::var("PUBLIC_CHANNEL_ID")
            .expect("Unable to read PUBLIC_CHANNEL_ID from ENV")
            .parse()
            .expect("Unable to parse PUBLIC_CHANNEL_ID as i64"),
    );

    let config = ConfigParameters{
        admins_group_id : admin_channel_id,
        public_channel_id,
    };

    let mut dialogStorage = Vec::<DialogInfo>::new();

    let addr: SocketAddr = std::env::var("HOST")
        .expect("Unable to read HOST from ENV")
        .parse()
        .expect("Unable to parse HOST as SocketAddr");

    let url = std::env::var("LISTEN_URL")
        .expect("Unable to read LISTEN_URL from ENV")
        .parse()
        .expect("Unable to parse LISTEN_URL as url");

    let listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, url))
        .await
        .expect("Couldn't setup webhook");

    let message_handler = Update::filter_message()
        .branch(dptree::filter(|msg: Message, cfg: ConfigParameters| {
            msg.chat.id == cfg.admins_group_id
        }).endpoint(admin_group_message))
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(start))
        .branch(dptree::case![State::AskQuestion].endpoint(ask_question));

    Dispatcher::builder(bot, message_handler)
    .dependencies(dptree::deps![InMemStorage::<State>::new(), config, Arc::new(Mutex::new(dialogStorage))])
    .enable_ctrlc_handler()
    .build()
    .dispatch_with_listener(
        listener,
        LoggingErrorHandler::with_custom_text("An error from the update listener"),
    )
    .await;
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Привет! Это бот для анонимных вопросов. Задавай вопрос! Только давай одним сообщением. Ок?",
    )
    .await?;
    dialogue.update(State::AskQuestion).await?;
    Ok(())
}

async fn ask_question(bot: Bot, dialogue: MyDialogue, msg: Message, cfg: ConfigParameters, storage: Arc<Mutex<Vec<DialogInfo>>>) -> HandlerResult {
    let message = msg.text().ok_or(anyhow!("Fail to read message"))?;
    bot.send_message(msg.chat.id, "Спасибо за вопрос!").await?;
    let send_messaeg = bot.send_message(cfg.admins_group_id, message).await?;
    let mut storage_unlocked = storage.lock().await;
    storage_unlocked.push(DialogInfo{
        source_channel_id: msg.chat.id,
        source_message_id: send_messaeg.id,
        source_in_channel_message_id: msg.id,
    });
    drop(storage_unlocked);
    dialogue.update(State::Start).await?;
    Ok(())
}

async fn admin_group_message(bot: Bot, msg: Message, cfg: ConfigParameters, storage: Arc<Mutex<Vec<DialogInfo>>>) -> HandlerResult {
    if let Some(original_message) = msg.reply_to_message() {
        let message = msg.text().ok_or(anyhow!("Fail to read message"))?;
        let original_forwarded = bot.forward_message(cfg.public_channel_id, original_message.chat.id, original_message.id).await?;
        let response = bot.send_message(cfg.public_channel_id, message).reply_to_message_id(original_forwarded.id).await?;

        let storage_unlocked = storage.lock().await;
        for info in storage_unlocked.iter() {
            if info.source_message_id == original_message.id {
                bot.forward_message(info.source_channel_id, response.chat.id, response.id).await?;
                break;
            }
        }
        drop(storage_unlocked);
    }
    Ok(())
}
