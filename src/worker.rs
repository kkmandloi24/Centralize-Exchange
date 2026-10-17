use crate::db::{get_asset_registry, resolve_asset_symbol, AssetBalance, AssetInfo, AssetSymbol, DbStore};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

/// Response payload containing updated balance details and metadata
#[derive(Debug, Clone)]
pub struct BalanceOpResult {
    pub updated_balance: AssetBalance,
    pub asset_info: AssetInfo,
}

/// All operations supported by the balance worker
pub enum BalanceCommand {
    Deposit {
        user_id: u32,
        asset_identifier: String,
        amount: f64,
        payment_type: crate::db::Type,
        resp: oneshot::Sender<Result<BalanceOpResult, String>>,
    },
    GetSingleBalance {
        user_id: u32,
        asset_identifier: String,
        resp: oneshot::Sender<Result<BalanceOpResult, String>>,
    },
    GetAllBalances {
        user_id: u32,
        resp: oneshot::Sender<HashMap<AssetSymbol, AssetBalance>>,
    },
    LockFunds{
        user_id: u32,
        asset_identifier: String,
        amount: f64,
        resp: oneshot::Sender<Result<(), String>>,
    },
    UnlockFunds {
        user_id: u32,
        asset_identifier: String,
        amount: f64,
        resp: oneshot::Sender<Result<(), String>>,
    },
}


/// Wallet creation function:  Generates a default zero-balance wallet for all registered assets
fn default_user_balances() -> HashMap<AssetSymbol, AssetBalance> {
    let mut wallet = HashMap::new();
    let registry = get_asset_registry();

    for asset in registry {
        let formatted_zero = format!("{:.1$}", 0.0, asset.decimals as usize);
        wallet.insert(
            asset.symbol,
            AssetBalance {
                available: formatted_zero.clone(),
                locked: formatted_zero,
            },
        );
    }
    wallet
}

/// Helper inside worker loop
fn get_or_create_user_wallet(
    db: &mut DbStore,
    user_id: u32,
) -> &mut HashMap<AssetSymbol, AssetBalance> {
    //first we check bu user-id entry if the user wallet in the db if not er tun default wallet creation
    db.entry(user_id).or_insert_with(default_user_balances)
}

//MAKING SINGLE THREADED Reciver (handlers points which recive request)
/// Dedicated single-threaded worker loop processing operations sequentially
pub async fn start_balance_worker(mut receiver: mpsc::Receiver<BalanceCommand>) {
    let mut db: DbStore = HashMap::new();

    while let Some(cmd) = receiver.recv().await {
        match cmd {
            BalanceCommand::Deposit {
                user_id,
                asset_identifier,
                amount,
                resp,
                ..
            } => {
                let res = (|| {
                    let asset_info = resolve_asset_symbol(&asset_identifier)
                        .ok_or_else(|| "We do not support this asset".to_string())?;

                    let wallet = get_or_create_user_wallet(&mut db, user_id);
                    let balance_entry = wallet
                        .get_mut(&asset_info.symbol)
                        .ok_or_else(|| "Asset balance entry not found".to_string())?;

                    let current_available: f64 = balance_entry.available.parse().unwrap_or(0.0);
                    let new_available = current_available + amount;

                    balance_entry.available =
                        format!("{:.1$}", new_available, asset_info.decimals as usize);

                    Ok(BalanceOpResult {
                        updated_balance: balance_entry.clone(),
                        asset_info,
                    })
                })();

                let _ = resp.send(res);
            }

            BalanceCommand::GetSingleBalance {
                user_id,
                asset_identifier,
                resp,
            } => {
                let res = (|| {
                    let asset_info = resolve_asset_symbol(&asset_identifier)
                        .ok_or_else(|| "Not supported the asset".to_string())?;

                    let wallet = get_or_create_user_wallet(&mut db, user_id);
                    let balance_entry = wallet
                        .get(&asset_info.symbol)
                        .cloned()
                        .unwrap_or_else(|| {
                            let zero = format!("{:.1$}", 0.0, asset_info.decimals as usize);
                            AssetBalance {
                                available: zero.clone(),
                                locked: zero,
                            }
                        });

                    Ok(BalanceOpResult {
                        updated_balance: balance_entry,
                        asset_info,
                    })
                })();

                let _ = resp.send(res);
            },

            BalanceCommand::GetAllBalances { user_id, resp } => {
                let wallet = get_or_create_user_wallet(&mut db, user_id).clone();
                let _ = resp.send(wallet);
            },

            ////////////////comands for lock and unlock//////
    BalanceCommand::LockFunds { user_id, asset_identifier, amount, resp } => {
    let res = (|| {
        let asset_info = resolve_asset_symbol(&asset_identifier)
            .ok_or_else(|| "Unsupported asset".to_string())?;

        let wallet = get_or_create_user_wallet(&mut db, user_id);
        let balance_entry = wallet
            .get_mut(&asset_info.symbol)
            .ok_or_else(|| "Wallet entry not found".to_string())?;

        let available: f64 = balance_entry.available.parse().unwrap_or(0.0);
        if available < amount {
            return Err("Insufficient available balance".to_string());
        }

        let locked: f64 = balance_entry.locked.parse().unwrap_or(0.0);

        // Update available and locked
        let new_available = available - amount;
        let new_locked = locked + amount;

        balance_entry.available = format!("{:.1$}", new_available, asset_info.decimals as usize);
        balance_entry.locked = format!("{:.1$}", new_locked, asset_info.decimals as usize);

        Ok(())
    })();
    let _ = resp.send(res);
}

BalanceCommand::UnlockFunds { user_id, asset_identifier, amount, resp } => {
    let res = (|| {
        let asset_info = resolve_asset_symbol(&asset_identifier)
            .ok_or_else(|| "Unsupported asset".to_string())?;

        let wallet = get_or_create_user_wallet(&mut db, user_id);
        let balance_entry = wallet
            .get_mut(&asset_info.symbol)
            .ok_or_else(|| "Wallet entry not found".to_string())?;

        let locked: f64 = balance_entry.locked.parse().unwrap_or(0.0);
        if locked < amount {
            return Err("Invalid unlock amount exceeds locked balance".to_string());
        }

        let available: f64 = balance_entry.available.parse().unwrap_or(0.0);

        // Move back from locked to available
        let new_locked = locked - amount;
        let new_available = available + amount;

        balance_entry.available = format!("{:.1$}", new_available, asset_info.decimals as usize);
        balance_entry.locked = format!("{:.1$}", new_locked, asset_info.decimals as usize);

        Ok(())
    })();
    let _ = resp.send(res);
}
            


        }
    }
}


