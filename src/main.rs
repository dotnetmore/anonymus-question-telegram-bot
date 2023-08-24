use std::sync::mpsc::channel;

use anyhow::anyhow;

use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    prelude::*,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    AskQuestion,
}

const channel_id: ChatId = ChatId(-1001284674171);

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to load .env file");
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();

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
    .dispatch()
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
    log::info!("Message: {}", message);
    bot.send_message(msg.chat.id, "Спасибо за вопрос!").await?;
    bot.send_message(channel_id, message).await?;
    dialogue.update(State::Start).await?;
    Ok(())
}
