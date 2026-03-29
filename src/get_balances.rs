use std::str::FromStr;

use anyhow::Result;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    pubkey::Pubkey
};
use teloxide::prelude::*;


pub async fn get_all_balance(client: RpcClient, owner_pubkey: Pubkey, bot: Bot, msg: Message) -> Result<String> {
    //. getting sol_balance
    let sol_balance: f64 = (client.get_balance(&owner_pubkey).await? as f64) / 1_000_000_000.0;
    let mut report_message = format!("First, your $SOL balance is {}", sol_balance);
    bot.send_message(msg.chat.id, "I'm starting get associated token accounts");

    let token_programs = [
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    ];

    for program_str in token_programs {
        let program_pubkey: Pubkey = Pubkey::from_str(program_str)?;
        let filter: TokenAccountsFilter = TokenAccountsFilter::ProgramId(program_pubkey);
        let accounts = client.get_token_accounts_by_owner(&owner_pubkey, filter).await?;

        for account in accounts {
            match account.account.data {
                solana_account_decoder::UiAccountData::Json(parsed_data) => {
                    let info = &parsed_data.parsed["info"];
                    
                    let mint = match info["mint"].as_str() {
                        Some(m) => m,
                        None => continue,
                    };

                    let token_amount = match info["tokenAmount"]["uiAmount"].as_f64() {
                        Some(amount) if amount > 0.0 => amount,
                        _ => continue, // Skipping dust
                    };

                    match get_token_price(mint).await {
                        Ok(price) => {
                            let usd_value = token_amount * price;
                            
                            match usd_value {
                                v if v > 0.01 => {
                                    report_message.push_str(&format!(
                                        "🔹 `{}`\n Balance: {:.2} | Price: ${:.4} | **So**: ${:.2}**\n\n",
                                        mint, token_amount, price, usd_value
                                    ));
                                },
                                _ => {}
                            }
                        },
                        Err(_) => {
                            report_message.push_str(&format!("`{}`\n Balance: {:.2} (Price not found)\n\n", mint, token_amount));
                        }
                    }
                },
                _ => continue, 
            }
        }
    }

    todo!()
}

async fn get_token_price(mint: &str) {
    todo!()
}
