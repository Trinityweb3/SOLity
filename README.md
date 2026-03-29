# 🦀 SOLity

A high-performance Solana helper bot built with **Rust**. It allows users to manage multiple wallets, check balances (SOL & SPL Tokens), and send transactions with real-time simulation and dynamic priority fees.

---

## ✨ Key Features

- **Wallet Management**: Securely link and organize multiple Solana private keys via Telegram.
- **Transfers**:
    - **Transaction Simulation**: Every transfer is simulated first to prevent failures and save fees.
    - **Dynamic Compute Limits**: Automatically calculates unit limits with a **1.3x safety buffer**.
    - **Priority Fees**: Integrated `set_compute_unit_price` to ensure transactions land during congestion.
- **Advanced Balance Tracking**(under development on 29 march):
    - **$SOL** balance monitoring(on .
    - **SPL Token Discovery**: Supports both **Token** and **SPL-Token-2022** programs.
    - **Dust Filtering**: Automatically hides accounts with negligible balances.
- **Explorer Integration**: Get direct **Solscan.io** links immediately upon transaction confirmation.

---

## 🛠 Tech Stack

*   **Framework**: [Teloxide](https://github.com) (Async Telegram Bot API)
*   **Blockchain**: [Solana SDK](https://github.com) & `solana-client`
*   **Database**: [SQLx](https://github.com) with **SQLite**
*   **Runtime**: [Tokio](https://tokio.rs) (Async ecosystem)

---

## 🚀 Installation & Setup

### 2. Environment Variables
Create a `.env` file in the project root:
```env
BOT_TOKEN=your_telegram_bot_token
DATABASE_URL=sqlite://sqlite.db
RPC_URL=https://api.mainnet-beta.solana.com (or custom)
```

### 2. Build and Run
```bash
cargo run --release
```

## ⚙️ How it Works
**Automated Transaction Safety**
- The bot uses **simulate_transaction** before sending. It ensures the recipient address is valid and calculates the exact units_consumed to avoid overpaying or failing

**Token & Price Discovery** (under development on 29 march)
- The bot iterates through all Associated Token Accounts (ATAs) across both standard and 2022 programs, identifying non-zero balances and preparing them for reporting

## Contacts
**Created by [@trinitycult](https://t.me)**
