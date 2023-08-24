use std::{net::SocketAddr, sync::mpsc::channel};

use anyhow::anyhow;

use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    prelude::*,
    update_listeners::webhooks,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AskQuestion,
}

fn get_channel_id() -> ChatId {
    return ChatId(
        std::env::var("CHANNEL_ID")
            .expect("Unable to read CHANNEL_ID from ENV")
            .parse()
            .expect("Unable to parse CHANNEL_ID as i64"),
    );
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to load .env file");
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();

    let channel_id = ChatId(
        std::env::var("CHANNEL_ID")
            .expect("Unable to read CHANNEL_ID from ENV")
            .parse()
            .expect("Unable to parse CHANNEL_ID as i64"),
    );

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

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::AskQuestion].endpoint(ask_question)),
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
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

async fn ask_question(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let message = msg.text().ok_or(anyhow!("Fail to read message"))?;
    bot.send_message(msg.chat.id, "Спасибо за вопрос!").await?;
    bot.send_message(get_channel_id(), message).await?;
    dialogue.update(State::Start).await?;
    Ok(())
}
