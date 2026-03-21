use std::str::FromStr; 
use anyhow::{Error, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction, instruction::Instruction, message::Message, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::{Keypair, Signer}, system_instruction, transaction::Transaction
};
use teloxide::prelude::*;
use dotenv::dotenv;

pub async fn send_sol(base58_private_key: String, reciever_addr_str: String, sol_count: String, bot: Bot, msg: teloxide::types::Message) -> Result<()> {
    dotenv().ok();
    let rpc_url: String = std::env::var("RPC_URL")?;
    let client: RpcClient = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let sender_keypair: Keypair = Keypair::from_base58_string(&base58_private_key);
    let reciever_adress: Pubkey = Pubkey::from_str(&reciever_addr_str.trim())?;

    let sol_f64: f64 = sol_count.parse()?;
    let lamports_amount: u64 = (sol_f64 * LAMPORTS_PER_SOL as f64 ) as u64;

    bot.send_message(msg.chat.id,format!("
        Sender: {}...\n
        Reciever: {}...\n
        Amount SOL: {}", 
        sender_keypair.pubkey(),
        reciever_adress,
        sol_f64)
    ).await?;

    let transfer_ix = system_instruction::transfer(
    &sender_keypair.pubkey(), 
    &reciever_adress, 
    lamports_amount
    );

    let compure_limit_u32: u32 = accounting_compute_limit(&client, &transfer_ix, &sender_keypair).await?;
    let compute_limit = ComputeBudgetInstruction::set_compute_unit_limit(compure_limit_u32);
    let priority_fee = ComputeBudgetInstruction::set_compute_unit_price(1_000);
    
    let recent_blockhash = client.get_latest_blockhash().await?;
    
    let tx = Transaction::new_signed_with_payer(
        &[compute_limit, priority_fee, transfer_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&tx).await?;
    
    bot.send_message(msg.chat.id,"Transaction confirmed!").await?;
    bot.send_message(msg.chat.id,format!("https://solscan.io/tx/{}", signature)).await?;

    Ok(())
}

pub async fn accounting_compute_limit(client: &RpcClient, transfer_ix: &Instruction, sender_keypair: &Keypair) -> Result<u32, Error> {
    let recent_blockhash = client.get_latest_blockhash().await?;

    let message = Message::new(
    &[transfer_ix.clone()], 
    Some(&sender_keypair.pubkey()),
    );
    let mut tx_to_simulate = Transaction::new_unsigned(message);
    tx_to_simulate.message.recent_blockhash = recent_blockhash;
    let simulation = client.simulate_transaction(&tx_to_simulate).await?;
    match simulation.value.err {
        Some(err) => return Err(anyhow::anyhow!(
            "Simulation failed: {:?}. Logs: {:?}", 
            err, 
            simulation.value.logs
        )),
        None => {}
    }

    let units_consumed = simulation.value.units_consumed.unwrap_or(1_000);
    let base_limit = (units_consumed as f64 * 1.3) as u32;
    let compure_limit_u32 = if base_limit < 1_000 {1_000} else {base_limit};
    return Ok(compure_limit_u32);
}
