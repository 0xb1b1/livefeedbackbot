extern crate pretty_env_logger;

mod database;

use database::Database;
use teloxide::{prelude::*,
               utils::command::BotCommands,
               dispatching::{
                 dialogue::InMemStorage,
                 Dispatcher
               },
              };

// // Dialogue management
// type DialogueStorage = Dialogue<State, InMemStorage<State>>;
// type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// #[derive(Clone, Default)]
// pub enum State {
//     #[default]
//     Start,
//     ReceiveAnswers {
//         start_code: String,
//         answers: Vec<String>
//     },
// }

pub async fn run() {
    pretty_env_logger::init();

    log::info!("Starting LiveFeedback Bot...");

    let bot = Bot::new("5641512811:AAG9-kmzU_oaQkoyuR5eDKTUaCLMU-qTwL4");

    // Database connection (postgres, port 5437, user: postgres, password: postgres)
    let db: Database = Database::new("postgres://postgres:postgres@127.0.0.1:5437/postgres").await.unwrap();
    db.init().await.unwrap();

    // TEST
    db.insert(database::Response {
        id: None,
        survey_name: "test".to_string(),
        telegram_id: 1,
    }).await.unwrap();

    Command::repl(bot, answer).await;

    // let handler = Update::filter_message()
    //     .enter_dialogue::<Message, InMemStorage<State>, State>()
    //     .branch(dptree::case![State::Start].endpoint(start))
    //     .branch(dptree::case![State::ReceiveAnswers { start_code, answers }].endpoint(receive_answers)
    // );

    // // Create Dispatcher for handling dialogues
    // Dispatcher::builder(
    //     bot,
    //     handler,
    // )
    // .dependencies(dptree::deps![InMemStorage::<State>::new()])
    // .enable_ctrlc_handler()
    // .build()
    // .dispatch()
    // .await;

    //Ok(())
}

async fn start(bot: Bot, dialogue: DialogueStorage, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            //? Check if message is at least 2 words long (TEMP)
            if text.split_whitespace().count() < 2 || !text.starts_with("/start") {
                bot.send_message(msg.chat.id, "Please send a text message that contains /start <code>").await?;
                return Ok(());
            }

            // Get start code from message
            let start_code = text.split_whitespace().nth(1).unwrap();

            // If the code is ADMIN, store admin in DB and use admin sequence
            if start_code == "ADMIN" {
                admin_panel(&bot, &msg).await?;
            }

            //? Send code to user
            bot.send_message(msg.chat.id, format!("Start code: {}", start_code)).await?;
            dialogue
                .update(State::ReceiveAnswers {
                    start_code: start_code.to_string(),
                    answers: Vec::new()
                })
                .await?;
            }
        None => {
            bot.send_message(msg.chat.id, "Please send a text message").await?;
        }
    };
    Ok(())
}

async fn receive_answers(bot: Bot,
                         dialogue: DialogueStorage,
                         (start_code, mut answers): (String, Vec<String>),
                         msg: Message) -> HandlerResult {
    // Get start code from message
    let answer = msg.text().unwrap();
    // If the command starts with /start, then exit current dialogue
    if answer.starts_with("/start") {
        bot.send_message(msg.chat.id, "Перезапуск диалога...").await?;
        dialogue.exit().await?;
        return Ok(());
    }
    if answer == "end" {
        // Send code to user
        bot.send_message(msg.chat.id, format!("Answers: {:?}", answers)).await?;
        dialogue.exit().await?;
        return Ok(());
    }
    // Send code to user
    bot.send_message(msg.chat.id, format!("Start code: {}\nAnswer: {}", start_code, answer)).await?;
    answers.push(answer.to_string());
    dialogue
        .update(State::ReceiveAnswers {
            start_code: start_code.to_string(),
            answers: answers
        })
        .await?;
    Ok(())
}

async fn admin_panel(bot: &Bot, msg: &Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Панель администратора\n\n").await?;
    Ok(())
}