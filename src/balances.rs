use core::f64;
use std::{collections::HashMap, str::FromStr};
use anyhow::Result;
use serde::Deserialize;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    pubkey::Pubkey
};

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

struct TokenInfo {
    ticker: String,
    price: f64,
    mcap: f64,
    liquidity: f64
}

pub async fn get_all_balance_and_return_hashmap(client: RpcClient, owner_pubkey: Pubkey) -> Result<HashMap<String, f64>> {
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
                    let vec_ticker_price_mcap_liquidity: TokenInfo = parse_api_response(&api_response_for_mint_address);

                    if vec_ticker_price_mcap_liquidity.ticker == "".to_string() {
                        continue;
                    };

                    let ticker: String = vec_ticker_price_mcap_liquidity.ticker;
                    
                    if vec_ticker_price_mcap_liquidity.price == 0.0 {
                        continue;
                    };

                    let price: f64 = vec_ticker_price_mcap_liquidity.price;
                    
                    let token_amount: f64 = match info["tokenAmount"]["uiAmount"].as_f64() {
                        Some(amount) if amount > 0.0 => amount,
                        _ => continue
                    };

                    let value_by_token: f64 = accounting_usdvalue_of_one_token(price, token_amount);

                    match map.get_mut(&ticker) {
                        Some(value) => {
                            *value = *value + value_by_token;
                        }
                        None => {
                            map.insert(ticker, value_by_token);
                        }
                    }    
                },
                _ => continue,
            }
        }
    }
    return Ok(map);
}

async fn get_api_response(mint: String) -> Result<String> {
    let path: String = format!("https://api.dexscreener.com/tokens/v1/solana/{}", mint);
    let response: reqwest::Response = reqwest::Client::new()
        .get(path)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("API Error"));
    }
    let api_response: String = response
        .text()
        .await?;

    return Ok(api_response);
}

fn parse_api_response(api_response: &str) -> TokenInfo {
    let mut ticker_price_mcap_liquidity: TokenInfo = TokenInfo {
        ticker: "".to_string(),
        price: 0.0,
        mcap: 0.0,
        liquidity: 0.0
    };

    let raws: Vec<CustomJson> = match serde_json::from_str(api_response) {
        Ok(r) => r,
        Err(_) => return ticker_price_mcap_liquidity
    };
    
    if raws.is_empty() {
        return ticker_price_mcap_liquidity;
    }

    let token_symbol: String = raws[0].base_token.symbol.clone();
    let token_name: String = raws[0].base_token.name.clone();
    let ticker: String = format!("{}({})", token_name, token_symbol);

    let price: f64= match raws[0].price_usd.clone().parse::<f64>() {
        Ok(p ) => p,
        Err(_) => 0.0
    };

    let mcap: f64= raws[0].market_cap;

    let liquidity: f64 = raws[0].liquidity.usd.clone();

    ticker_price_mcap_liquidity = TokenInfo { 
        ticker: ticker, 
        price: price, 
        mcap: mcap, 
        liquidity: liquidity 
    };
    

    return ticker_price_mcap_liquidity;
}

fn accounting_usdvalue_of_one_token(price: f64, amount: f64) -> f64 {
    let token_balance: f64 = amount * price;

    return token_balance
}

