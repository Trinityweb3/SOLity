use std::{env, sync::Arc};

use anyhow::Result;
use rand::{RngCore, rngs::OsRng};

use teloxide::{
    dispatching::{
        dialogue::{Dialogue, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    utils::command::BotCommands,
};

use dptree::case;

use sqlx::{Pool, Sqlite, Row};
use sqlx::sqlite::SqlitePool;

use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use base64::{engine::general_purpose, Engine};

mod sol_transfer;

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
    WaitingPrivateKey,
    WaitingSend,
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), anyhow::Error>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Commands {
    Start,
    Addwallet,
    Sendsol,
    Mywallets,
}

fn get_key() -> [u8; 32] {
    let key_str = env::var("SECRET_KEY").expect("SECRET_KEY required");
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_str.as_bytes()[0..32]);
    key
}

fn encrypt(data: &str) -> Option<String> {
    let key = get_key();
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);

    let encrypted = match cipher.encrypt(Nonce::from_slice(&nonce), data.as_bytes()) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let mut result = nonce.to_vec();
    result.extend(encrypted);

    Some(general_purpose::STANDARD.encode(result))
}

fn decrypt(data: &str) -> Option<String> {
    let key = get_key();
    let cipher = Aes256Gcm::new(&key.into());

    let decoded = match general_purpose::STANDARD.decode(data) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted = match cipher.decrypt(nonce, cipher_bytes) {
        Ok(v) => v,
        Err(_) => return None,
    };

    match String::from_utf8(decrypted) {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    cmd: Commands,
) -> HandlerResult {
    match cmd {
        Commands::Start => {
            bot.send_message(msg.chat.id, "Welcome").await?;
        }

        Commands::Addwallet => {
            bot.send_message(msg.chat.id, "Send private key").await?;
            dialogue.update(State::WaitingPrivateKey).await?;
        }

        Commands::Sendsol => {
            bot.send_message(msg.chat.id, "Enter: address amount").await?;
            dialogue.update(State::WaitingSend).await?;
        }

        Commands::Mywallets => {
            bot.send_message(msg.chat.id, "Wallets stored in DB").await?;
        }
    }

    Ok(())
}

async fn handle_private_key(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    pool: Arc<Pool<Sqlite>>,
) -> HandlerResult {

    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    let encrypted = match encrypt(text) {
        Some(v) => v,
        None => return Ok(()),
    };

    sqlx::query("INSERT INTO wallets (user_id, private_key) VALUES (?, ?)")
        .bind(msg.chat.id.0)
        .bind(encrypted)
        .execute(&*pool)
        .await?;

    bot.send_message(msg.chat.id, "Saved").await?;

    dialogue.update(State::Start).await?;
    Ok(())
}

async fn handle_send(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    pool: Arc<Pool<Sqlite>>,
) -> HandlerResult {

    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() != 2 {
        bot.send_message(msg.chat.id, "Format: address amount. You have to have uploaded private key. Use /addwallet at first").await?;
        return Ok(());
    }

    let to = parts[0].to_string();
    let amount = parts[1].to_string();

    let row = match sqlx::query(
        "SELECT private_key FROM wallets WHERE user_id = ? LIMIT 1"
    )
    .bind(msg.chat.id.0)
    .fetch_optional(&*pool)
    .await?
    {
        Some(r) => r,
        None => {
            bot.send_message(msg.chat.id, "No wallet found").await?;
            return Ok(());
        }
    };

    let encrypted_key: String = row.get("private_key");

    let private_key = match decrypt(&encrypted_key) {
        Some(k) => k,
        None => {
            bot.send_message(msg.chat.id, "Decrypt error").await?;
            return Ok(());
        }
    };

    bot.send_message(msg.chat.id, "Sending...").await?;

    match sol_transfer::send_sol(
        private_key,
        to,
        amount,
        bot.clone(),
        msg.clone(),
    )
    .await
    {
        Ok(_) => {}
        Err(_) => {
            bot.send_message(msg.chat.id, "Transaction failed").await?;
        }
    }

    dialogue.update(State::Start).await?;
    Ok(())
}

fn schema() -> UpdateHandler<anyhow::Error> {
    Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(
            dptree::entry()
                .filter_command::<Commands>()
                .endpoint(handle_command),
        )
        .branch(case![State::WaitingPrivateKey].endpoint(handle_private_key))
        .branch(case![State::WaitingSend].endpoint(handle_send))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let token: String = std::env::var("BOT_TOKEN")?;

    let bot: Bot = Bot::new(token);

    let db = env::var("DATABASE_URL")?;
    let pool = Arc::new(SqlitePool::connect(&db).await?);

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            pool
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
