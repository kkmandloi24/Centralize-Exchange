
use crate::db::{AssetBalance, AssetSymbol, AssetInfo,TransactionRecord};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;


// #[derive(Debug, Clone, Serialize, Deserialize)]
// // pub struct AssetBalance {
// //     pub available: String,
// //     pub locked: String,
// // }


//for all assets
#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    pub user_id: u32,
    pub balances: HashMap<AssetSymbol, AssetBalance>,
}


//for individual asset
#[derive(Serialize,Deserialize)]
pub struct SingleAssetResponse {
    pub user_id: u32,
    pub asset: AssetInfo,
    pub balance: AssetBalance,

}


#[derive(Serialize,Deserialize)]
pub struct TransactionResponse {
    pub transaction_log:TransactionRecord,
    pub user_id: u32,
    pub asset: AssetInfo,
    pub balance: AssetBalance,

}



#[derive(Serialize,Deserialize)]
pub struct OnrampRequest {
    pub amount: u32,
    pub payment_method: Option<String>
}


#[derive(Serialize,Deserialize)]
pub struct DepositRequest{
    pub amount:u32,
    pub asset:String 
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub message: String,
}



