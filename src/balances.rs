use core::f64;
use std::{collections::HashMap, fmt::format, str::FromStr};
use anyhow::Result;
use serde::Deserialize;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    pubkey::Pubkey
};

pub async fn get_all_balance(client: RpcClient, owner_pubkey: Pubkey) -> Result<HashMap<String, f64>> {
    let mut map: HashMap<String, f64> = HashMap::new();
    //. getting sol_balance
    let sol_balance: f64 = (client.get_balance(&owner_pubkey).await? as f64) / 1_000_000_000.0;
    map.insert("SOL".to_string(), sol_balance);


    let token_programs: [&str; 2] = [
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
                    
                    let mint: String = match info["mint"].as_str() {
                        Some(m) => m.to_string(),
                        None => continue,
                    };

                    let api_response_for_mint_address: String = get_api_response(mint).await?;
                    let vec_ticker_price_mcap_liquidity: Vec<String> = parse_api_response(&api_response_for_mint_address);
                    let ticker: String = vec_ticker_price_mcap_liquidity[0].clone();
                    let price: String = vec_ticker_price_mcap_liquidity[1].clone();
                    
                    let token_amount: f64 = match info["tokenAmount"]["uiAmount"].as_f64() {
                        Some(amount) if amount > 0.0 => amount,
                        _ => continue
                    };

                    let value_by_token: f64 = accounting_usdvalue_of_one_token(price, token_amount);

                    map.insert(ticker, value_by_token);
                },
                _ => continue,
            }
        }
    }
    return Ok(map);
}


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomJson {
    price_usd: String, 
    base_token: BaseToken,
    liquidity: Liquidity,
    market_cap: f64,
}

#[derive(Deserialize, Debug)]
struct Liquidity {
    usd: f64,
    base: i64,
    quote: f64
}

#[derive(Deserialize, Debug)]
struct BaseToken {
    address: String,
    name: String,
    symbol: String
}


async fn get_api_response(mint: String) -> Result<String> {
    let path: String = format!("https://api.dexscreener.com/tokens/v1/solana/{}", mint);
    let api_response: String = reqwest::Client::new()
        .get(path)
        .send()
        .await?
        .text()
        .await?;

    return Ok(api_response);
}

fn parse_api_response(api_response: &str) -> Vec<String> {
    let mut vec_ticker_price_mcap_liquidity: Vec<String> = Vec::new();

    let raws: Vec<CustomJson> = serde_json::from_str(api_response).unwrap();
    
    let price: String = raws[0].price_usd.clone();
    let token_name: String = raws[0].base_token.name.clone();
    let liquidity: String = raws[0].liquidity.usd.clone().to_string();
    let token_symbol: String = raws[0].base_token.symbol.clone();
    let ticker: String = format!("{}({})", token_name, token_symbol);
    let mcap: String = raws[0].market_cap.to_string();

    vec_ticker_price_mcap_liquidity.push(ticker);
    vec_ticker_price_mcap_liquidity.push(price);
    vec_ticker_price_mcap_liquidity.push(mcap);
    vec_ticker_price_mcap_liquidity.push(liquidity);

    return vec_ticker_price_mcap_liquidity;
}

fn accounting_usdvalue_of_one_token(price: String, amount: f64) -> f64 {
    let price: f64 = price.parse::<f64>().unwrap();
    let token_balance: f64 = amount * price;

    return token_balance
}

