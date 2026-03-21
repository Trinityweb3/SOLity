mod sol_transfer;
use std::sync::Arc;
use sqlx::Row;
use anyhow::Error;
use sqlx::{Pool, Sqlite};
use teloxide::dispatching::dialogue::GetChatId;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use teloxide::prelude::*;

use sqlx::sqlite::SqlitePool;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Commands {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Send $SOL")]
    Send_sol,
    #[command(description = "Add a wallet")]
    Add_a_wallet,
    #[command(description = "Add a wallet")]
    My_Wallets
}

const UNKNOWN_COMMAND: &str = "Unknown command";

const ADD_A_WALLET_MESSAGE: &str = "Ok. Please enter a your private key (87-88 characters)";

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let bot_token = std::env::var("BOT_TOKEN").unwrap();
    let database_url = std::env::var("DATABASE_URL").unwrap();
    let pool = SqlitePool::connect(&database_url).await?;
    let pool = Arc::new(pool);
    let bot: Bot = Bot::new(bot_token);

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let pool = Arc::clone(&pool);
        async move {

            match Commands::parse(msg.text().unwrap_or("/start"), "SOLity_tgbot") {
                Ok(cmd) => {
                    answer(bot, msg, cmd, pool).await?;
                },
                Err(_) => {
                    let text: &str = msg.text().unwrap_or("/start");
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    if parts.len() == 3 {
                        let private_key: String = parts[0].to_string();
                        let to_address: String = parts[1].to_string();
                        let amount: String = parts[2].to_string();

                        match sol_transfer::send_sol(private_key, to_address, amount, bot.clone(), msg.clone()).await {
                            Ok(_) => {},
                            Err(_) => {
                                bot.send_message(msg.chat.id, "Rare error occure").await.ok();
                            }
                        };
                        return respond(());
                    }

                    if text.len() >= 87 && text.len() < 89 {
                        sqlx::query("INSERT INTO wallets (user_id, private_key) VALUES (?, ?)")
                            .bind(msg.chat.id.0)
                            .bind(text)
                            .execute(&*pool)
                            .await
                            .ok();
                        bot.send_message(msg.chat.id, format!("</b>Private key recieved</b>\nTry /my_wallets or add an another wallet /add_a_wallet")).parse_mode(ParseMode::Html).await?;
                    } else {
                        bot.send_message(msg.chat.id, UNKNOWN_COMMAND).await?;
                    }
                }
            }
            respond(())
        }
    }).await;
    Ok(())
}

async fn answer(bot: Bot, msg: Message, cmd: Commands, pool: Arc<Pool<Sqlite>>) -> ResponseResult<()> {
    match cmd {
        Commands::Start => {
            bot.send_message(msg.chat.id, format!("<b>Hey!</b>\nI'm the your personal solana helper\n<b>Available commands:</b>\n/start\n/add_a_wallet\n/send_sol\n/my_wallets\n\nThe code's fully open-source and available on the GitHub. Follow the link -  https://github.com/Trinityweb3/helper_bot\n<b>Created by @trinitycult</b>")).parse_mode(ParseMode::Html).await?;
        },
        Commands::My_Wallets => {
            let private_key_rows = sqlx::query("
                    SELECT private_key FROM wallets WHERE user_id = ?"
                )
                .bind(msg.chat.id.0)
                .fetch_all(&*pool)
                .await
                .unwrap();
            bot.send_message(msg.chat.id, "<b>Your private keys</b> 👇").await?;
            for row in private_key_rows {
                let private_key: String = row.get("private_key");
                bot.send_message(msg.chat.id, format!("<code>{}</code>", private_key)).parse_mode(ParseMode::Html).await?;
            }
        },
        Commands::Add_a_wallet => {
            bot.send_message(msg.chat.id, ADD_A_WALLET_MESSAGE).await?;
        },
        Commands::Send_sol => {
            bot.send_message(msg.chat.id, "Ok\n\n<b>Enter the private key from which you'll send $SOL</b> (You can look added keys by the /my_wallets)\n<b>Enter the sol address where you wanna send $SOL</b>\n<b>Enter the $SOL amount</b>\n\nFollow the format: <b>private_key sol_address SOL_amount</b>").parse_mode(ParseMode::Html).await?;
        }
    }
    Ok(())
}
