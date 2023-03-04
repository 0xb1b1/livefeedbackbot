extern crate pretty_env_logger;

mod database;
mod wcsv;

use database::{Database, Response, User, CodeResult, FullResponse};
use wcsv::createCSVBody;
use async_once::AsyncOnce;
//use futures::SinkExt;
use dotenvy::dotenv;
use std::env;
use teloxide::{prelude::*,
               utils::command::BotCommands,
               types::{ChatId, InputFile},
              };

lazy_static! {
    /// A singleton database with a pool connection
    /// that can be shared between threads
    static ref DATABASE: AsyncOnce<Database> = AsyncOnce::new(async {
        Database::new(env::var("PGSQL_ADDR").unwrap().as_str())
            .await
            .unwrap_or_else(|err| panic!("Failed to connect to database: {}", err))
    });
}

pub async fn run() {
    pretty_env_logger::init();
    if env::var("LIVEFEEDBACK_DOCKER").unwrap_or_else(|_| "false".to_string()) != "true" {
        dotenv().ok();
    }

    log::info!("Starting LiveFeedback Bot...");

    // Database connection
    //let db: Database = Database::new("postgres://postgres:postgres@127.0.0.1:5437/postgres").await.unwrap();

    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").unwrap());

    // Initialize database
    DATABASE.get().await.init().await.unwrap();

    let db = DATABASE.get().await;
    Command::repl(bot, move |bot, msg, cmd| answer(bot, msg, cmd, db)).await;

    //Ok(())
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "camelCase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Start the bot with a speech code `<code>`")]
    Start(String),
    #[command(description = "Display this help message `None`")]
    Help,
    #[command(description = "Display information about the bot and its author `None`")]
    About,
    #[command(description = "List all participants by code `<secret> <code>`", parse_with = "split")]
    ListByCode { secret: String, code: String },
    #[command(description = "List all participants `<secret>`")]
    ListAll(String),
    #[command(description = "List all participants in a CSV document `<secret>`")]
    ListAllCSV(String),
    #[command(description = "Get all your responses `None`")]
    Responses,
    #[command(description = "Add allowed speech code `<secret> <code>`", parse_with = "split")]
    AddCode { secret: String, code: String },
    #[command(description = "Delete allowed speech code `<secret> <code>`", parse_with = "split")]
    DelCode { secret: String, code: String },
    #[command(description = "Flush all responses (DANGEROUS!) `<secret> YES`", parse_with = "split")]
    FlushResponses { secret: String, confirmation: String },
    #[command(description = "Flush all allowed speech codes AND ALL RESPONSES (DANGEROUS!) `<secret> YES`", parse_with = "split")]
    FlushCodes { secret: String, confirmation: String },
    #[command(description = "Get all allowed codes `<secret>`")]
    Codes(String),
    #[command(description = "Broadcast a message to all users `<secret> <message>`", parse_with = "default")]
    Broadcast(String),
    #[command(description = "Broadcast a message to code responders `<secret> <code> <message>`", parse_with = "default")]
    BroadcastToCode(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command, db: &Database) -> ResponseResult<()> {
    let user: User = User {
        telegram_id: *(&msg.chat.id.to_string().parse::<i32>().unwrap()),
        username: msg.from().unwrap().username.clone().unwrap_or_else(|| "".to_string()),
        first_name: msg.from().unwrap().first_name.clone(),
        last_name: msg.from().unwrap().last_name.clone().unwrap_or_else(|| "".to_string()),
    };

    match cmd {
        Command::Start(code) => {
            start(bot, user, code.to_uppercase(), db).await?;
            ()
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
            ()
        }
        Command::About => {
            bot.send_message(msg.chat.id, "Разработкой занимался Аксель (@oxb1b1) из ITAM (@itatmisis) ;)
Исходный код бота в открытом доступе на GitHub: https://github.com/0xb1b1/livefeedbackbot
\nRust <3").await?;
            ()
        }
        Command::ListByCode { secret, code } => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            // List all participants by code
            list_by_code(&bot, msg.chat.id, code.to_uppercase(), db).await?;
            ()
        }
        Command::ListAll(secret) => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            // List all participants
            list_all(bot, msg.chat.id, db).await?;
            ()
        }
        Command::ListAllCSV(secret) => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            // List all participants
            list_all_csv(bot, msg.chat.id, db).await?;
            ()
        }
        Command::Responses => {
            // List all responses by user
            user_responses(bot, msg.chat.id, db).await?;
            ()
        }
        Command::AddCode { secret, code } => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            add_code(bot, msg.chat.id, code.to_uppercase(), db).await?;
            ()
        }
        Command::DelCode { secret, code } => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            del_code(bot, msg.chat.id, code.to_uppercase(), db).await?;
            ()
        }
        Command::FlushResponses { secret, confirmation } => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            if confirmation != "YES" {
                bot.send_message(msg.chat.id, "Операция не подтверждена. Отмена").await?;
                return Ok(());
            }
            flush_responses(bot, msg.chat.id, db).await?;
            ()
        }
        Command::FlushCodes { secret, confirmation } => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            if confirmation != "YES" {
                bot.send_message(msg.chat.id, "Неверное подтверждение").await?;
                return Ok(());
            }
            flush_codes(bot, msg.chat.id, db).await?;
            ()
        }
        Command::Codes(secret) => {
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            let codes = db.get_codes().await.unwrap();
            let codes = codes.iter().map(|c| c.clone()).collect::<Vec<String>>().join(", ");
            bot.send_message(msg.chat.id, format!("Allowed codes: {}", codes)).await?;
            ()
        }
        Command::Broadcast(combined) => {
            // Split secret and message
            let mut split = combined.splitn(2, ' ');
            let secret = split.next().unwrap_or_default().to_owned();
            let message = split.next().unwrap_or_default().to_owned();
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            if message == "" {
                bot.send_message(msg.chat.id, "Неверный формат сообщения").await?;
                return Ok(());
            }
            broadcast(bot, msg.chat.id, message, db, None).await?;
            ()
        }
        Command::BroadcastToCode(combined) => {
            // Split combined into secret, code, and message
            let mut split = combined.splitn(3, ' ');
            let secret = split.next().unwrap_or_default().to_owned();
            let code = split.next().unwrap_or_default().to_owned();
            let message = split.next().unwrap_or_default().to_owned();
            if secret != env::var("SECRET").unwrap() {
                bot.send_message(msg.chat.id, "Неверный секретный код").await?;
                return Ok(());
            }
            if message == "" {
                bot.send_message(msg.chat.id, "Неверный формат сообщения").await?;
                return Ok(());
            }
            broadcast(bot, msg.chat.id, message, db, Some(code.to_uppercase())).await?;
            ()
        }
    };

    Ok(())
}

async fn start(bot: Bot, user: User, code: String, db: &Database) -> ResponseResult<()> {
    let chat_id: ChatId = ChatId(user.telegram_id as i64);
    if code == String::from("") {
        bot.send_message(chat_id,
            "Вас приветствует LiveFeedback бот! Разработкой занимался Аксель (@oxb1b1) из ITAM (@itatmisis) ;)
Исходный код бота в открытом доступе. Узнать больше: /about
\nПожалуйста, введите код выступления после команды /start или отсканируйте QR на мероприятии
Помощь: /help").await?;
        return Ok(());
    }
    if !db.is_code_allowed(&code).await.unwrap() {
        bot.send_message(chat_id, "Код не найден").await?;
        return Ok(());
    }
    db.add_user(&user).await.unwrap();
    db.insert(Response {
        id: None,
        speech_code: code.clone(),
        telegram_id: user.telegram_id
    }).await.unwrap();

    bot.send_message(chat_id, format!("Спасибо! Мы записали, что вы были на выступлении {}\n\nПомощь: /help", code)).await?;
    Ok(())
}

async fn add_code(bot: Bot, chat_id: ChatId, code: String, db: &Database) -> ResponseResult<()> {
    db.add_code(&code).await.unwrap();
    bot.send_message(chat_id, format!("Код {} добавлен", code)).await?;
    Ok(())
}

async fn del_code(bot: Bot, chat_id: ChatId, code: String, db: &Database) -> ResponseResult<()> {
    if !db.is_code_allowed(&code).await.unwrap() {
        bot.send_message(chat_id, "Код не найден").await?;
        return Ok(());
    }
    db.del_code(&code).await.unwrap();
    bot.send_message(chat_id, format!("Код {} удален", code)).await?;
    Ok(())
}

async fn flush_responses(bot: Bot, chat_id: ChatId, db: &Database) -> ResponseResult<()> {
    db.flush_responses().await.unwrap();
    bot.send_message(chat_id, "Все отклики удалены").await?;
    Ok(())
}

async fn flush_codes(bot: Bot, chat_id: ChatId, db: &Database) -> ResponseResult<()> {
    db.flush_codes().await.unwrap();
    bot.send_message(chat_id, "Все коды выступлений и участники удалены").await?;
    Ok(())
}

async fn list_all(bot: Bot, chat_id: ChatId, db: &Database) -> ResponseResult<()> {
    for code in db.get_codes().await.unwrap() {
        list_by_code(&bot, chat_id, code, db).await?;
    }

    Ok(())
}

async fn list_all_csv(bot: Bot, chat_id: ChatId, db: &Database) -> ResponseResult<()> {
    let coderes = createCSVBody(db.get_all_code_results().await.unwrap());
    let teloxdoc = InputFile::memory(coderes.into_bytes());
    bot.send_document(chat_id, teloxdoc).await.ok();
    Ok(())
}

async fn list_by_code(bot: &Bot, chat_id: ChatId, code: String, db: &Database) -> ResponseResult<()> {
    let responses = db.vec_response_to_fullresponse(db.get_by_code(code.clone()).await.unwrap()).await.unwrap();
    let responses_count: i32 = responses.len() as i32;
    // Format responses as a string for output in chatbot
    let responses = responses.iter().map(|r| format!("@{} — {} {}", r.username, r.first_name, r.last_name)).collect::<Vec<String>>().join("\n");
    bot.send_message(chat_id, format!("На выступлении {} отметились {} человек(а):\n\n{}", code, responses_count, responses)).await?;

    Ok(())
}

async fn user_responses(bot: Bot, chat_id: ChatId, db: &Database) -> ResponseResult<()> {
    let responses = db.get_by_user(chat_id.to_string().parse::<i32>().unwrap()).await.unwrap();
    let responses_count: i32 = responses.len() as i32;
    // Format responses as a string for output in chatbot
    let responses = responses.iter().map(|r| r.speech_code.clone()).collect::<Vec<String>>().join(", ");
    bot.send_message(chat_id, format!("Вы отметились на {} выступлениях:\n\n{}", responses_count, responses)).await?;

    Ok(())
}


async fn broadcast(bot: Bot, chat_id: ChatId, message: String, db: &Database, scope: Option<String>) -> ResponseResult<()> {
    bot.send_message(chat_id, "Начинаю рассылку всем пользователям...").await?;
    let respondents = match scope {
        Some(scope) => db.get_users_by_code(scope).await.unwrap(),
        None => db.get_users().await.unwrap()
    };
    // Send message to all respondents
    for respondent in &respondents {
        bot.send_message(ChatId(*respondent as i64), message.clone()).await.ok();
    }
    bot.send_message(chat_id, format!("Сообщение успешно отправлено количеству людей: {}", respondents.len())).await?;
    Ok(())
}
