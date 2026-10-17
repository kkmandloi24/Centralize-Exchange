use crate::worker::BalanceCommand;
use tokio::sync::mpsc;


use crate::models::users::User;
use crate::engine::{ExchangeServer};
use crate::wal::{Wal};


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;


// ==========================================
// 1. ASSET METADATA & REGISTRY
// ==========================================


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
   pub id: u32,
   pub symbol: AssetSymbol,
   pub name: String,
   pub decimals: u8,
}


// Global lookup helper for supported assets on the exchange
pub fn get_asset_registry() -> Vec<AssetInfo> {
   vec![
       AssetInfo { id: 1, symbol:AssetSymbol::USD,  name: "US Dollar".into(), decimals: 2 },
       AssetInfo { id: 2, symbol: AssetSymbol::USDT, name: "Tether".into(),    decimals: 6 },
       AssetInfo { id: 3, symbol: AssetSymbol::USDC, name: "USD Coin".into(),  decimals: 6 },
       AssetInfo { id: 4, symbol: AssetSymbol::SOL,  name: "Solana".into(),    decimals: 9 },
       AssetInfo { id: 5, symbol: AssetSymbol::ETH,  name: "Ethereum".into(),  decimals: 18 },
   ]
}


// Helper: Finds symbol by numeric ID or string symbol
pub fn resolve_asset_symbol(identifier: &str) -> Option<AssetInfo> {
   let registry = get_asset_registry();


   // 1. If identifier is numeric (e.g. "1"), find by ID
   if let Ok(asset_id) = identifier.parse::<u32>() {
       return registry.into_iter().find(|a| a.id == asset_id);
   }


   // 2. Otherwise, find by symbol name (e.g. "USD"), use method of as_str then we implemented on AssetSymbol
   registry.into_iter().find(|a| a.symbol.as_str().eq_ignore_ascii_case(identifier))
}


// ==========================================
// 2. BALANCE & DATABASE STRUCTURES
// ==========================================


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
   pub available:String,
   pub locked: String,
}




//typ of DbStore which stores user balance
// Maps: user_id (u32) -> (AssetSymbol String -> AssetBalance)
pub type DbStore = HashMap<u32, HashMap<AssetSymbol, AssetBalance>>;



pub struct AppState {
   pub user_index: Mutex<u32>,
   pub users: Mutex<Vec<User>>,
   pub balance_tx: mpsc::Sender<BalanceCommand>, // Replacing Mutex<DbStore> //this avoiding race condition
   pub txn_log_db:Mutex<TransactionDb>,
   pub server:Mutex<ExchangeServer>,
   pub wal:Mutex<Wal>
}




// impl AppState {
//    pub fn init() -> Self {
//        AppState {
//            user_index: Mutex::new(1),
//            users: Mutex::new(Vec::new()),
//            db: Mutex::new(HashMap::new()),
//            txn_log_db:Mutex::new(TransactionDb::new()),
//        }
//    }
// }


/////////////
/// 
/// 
/// 
impl AppState {
    // Accept server and wal as parameters during initialization
    pub fn init(
        balance_tx: mpsc::Sender<BalanceCommand>,
        server: ExchangeServer,
        wal: Wal,
    ) -> Self {
        AppState {
            user_index: Mutex::new(1),
            users: Mutex::new(Vec::new()),
            balance_tx,
            txn_log_db: Mutex::new(TransactionDb::new()),
            server: Mutex::new(server), // <--- Wrap server in Mutex
            wal: Mutex::new(wal),       // <--- Wrap wal in Mutex
        }
    }
}

//old codecargo run

// impl AppState {
//     // Pass the mpsc sender here during initialization
//     pub fn init(balance_tx: mpsc::Sender<BalanceCommand>) -> Self {
//         AppState {
//             user_index: Mutex::new(1),
//             users: Mutex::new(Vec::new()),
//             balance_tx, // Use balance_tx instead of db
//             txn_log_db: Mutex::new(TransactionDb::new()),

//         }
//     }
// }
////////////







// ==========================================
// 3. DEFAULT WALLET GENERATION
// ==========================================


/// Generates a default zero-balance wallet for all registered assets
pub fn default_user_balances() -> HashMap<AssetSymbol, AssetBalance> {
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


/// Helper function to get existing wallet or create a default one if new
pub fn get_or_create_user_wallet(
   db: &mut DbStore,
   user_id: u32,
) -> &mut HashMap<AssetSymbol, AssetBalance> {
   db.entry(user_id).or_insert_with(default_user_balances)
}


////////////
//Then we realise we have 3 key those requires the more than one variant like type, assetSymbol, status So first we defined enum for them, as
#[derive(Debug,Clone,Serialize, Deserialize)]
pub enum Type{
   OnRamp,
   Deposit,
   Withdrawal,
   Others,


}


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetSymbol{
   USD,
   USDT,
   USDC,
   SOL,
   ETH,
}

//we use below as_str in the asset resolve function
//Now we implement a method in the AssetSmbol which will turn the USD to 'USD" a string USD, then we use it asset resolve function
impl AssetSymbol { pub fn as_str(&self) -> &str { match self { AssetSymbol::USD => "USD", AssetSymbol::USDT => "USDT", AssetSymbol::USDC => "USDC", AssetSymbol::SOL => "SOL", AssetSymbol::ETH => "ETH", } } }


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status{
   Successful,
   Failed,
   Pending,
}

// Then we created the Struct for the TransactionRecord: 


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord{
pub id: String,
 pub user_id: u32,
 pub payment_type: Type,
 pub asset_symbol: AssetSymbol,
 pub payment_method: Option<String>,
 pub amount: usize,
 pub status: Status,
 pub created_at: String,


}

//Now we create DB for these all : here Db will be aslo a variable in memory so also deine one struct which involve all above struc enums in a structuredway, so need again one struct = TransactionDb struct which will create trasaction_log Db variable(database),

#[derive(Serialize, Deserialize)]
pub struct TransactionDb {
    pub transactions: HashMap<u32, HashMap<String, TransactionRecord>>,
}

impl TransactionDb{
   // Initialize an empty transaction database 
   pub fn new() -> Self { Self { transactions: HashMap::new(), } } 

   // Add a transaction 
   pub fn add_transaction( &mut self, user_id: u32, transaction: &TransactionRecord, ) { 
    self.transactions .entry(user_id) .or_default() .insert(transaction.id.clone(), transaction.clone()); }
     
     // Delete a transaction 
     pub fn delete_transaction( &mut self, user_id: u32, transaction_id: &str, ) -> Option<TransactionRecord> { self.transactions .get_mut(&user_id) .and_then(|user_transactions| { user_transactions.remove(transaction_id) }) }
     
      // Get one transaction 
      pub fn get_transaction( &self, user_id: u32, transaction_id: &str, ) -> Option<&TransactionRecord> { self.transactions .get(&user_id) .and_then(|user_transactions| { user_transactions.get(transaction_id) }) } 
      
      // Get all transactions of a user 
      pub fn get_user_transactions( &self, user_id: u32, ) -> Option<&HashMap<String, TransactionRecord>> { self.transactions.get(&user_id) } 
      
      // Update an existing transaction 
      pub fn update_transaction( &mut self, user_id: u32, transaction: TransactionRecord, ) -> Option<TransactionRecord> { self.transactions .get_mut(&user_id) .and_then(|user_transactions| { user_transactions.insert(transaction.id.clone(), transaction) }) } 
      
      // Check whether a transaction exists 
      pub fn transaction_exists( &self, user_id: u32, transaction_id: &str, ) -> bool { self.get_transaction(user_id, transaction_id).is_some() } 

}





///////////////

//Method 
//initilize the empty db
// impl TransactionDb {
//    // Create an empty transaction database //Initializing
//   pub fn new() -> Self { Self { transactions: HashMap::new(), } }

//   //Add transaction method
//   pub fn add_transaction(
//         &mut self,
//         user_id: u32,
//         transaction: &TransactionRecord,
//     ) {
//         self.transactions
//             .entry(user_id)
//             .or_default()
//             .insert(transaction.id.clone(), transaction.clone());
//     }

//     // //Delete Transaction Records
//     // pub fn delete_transaction( &mut self, user_id: u32, transaction_id: &str, ) -> Option<TransactionRecord> {
//     //    self.transactions.get_mut(&user_id).and_then(|user_transactions| { user_transactions.remove(transaction_id) })

//      //Get One transaction
//     pub fn get_transaction( &self, user_id: u32, transaction_id: &String, ) -> Option<&TransactionRecord> { self.transactions .get(&user_id) .and_then(|user_transactions| { user_transactions.get(transaction_id) }) }

//     //Get All Transaction of a user
//     pub fn get_user_transactions( &self, user_id: u32, ) -> Option<&HashMap<String, TransactionRecord>> { self.transactions.get(&user_id) }


//     //Update existing transaction record
//     pub fn update_transaction( &mut self, user_id: u32, transaction: TransactionRecord, ) -> Option<TransactionRecord> { self.transactions .get_mut(&user_id) .and_then(|user_transactions| { user_transactions.insert(transaction.id.clone(), transaction) }) }

//     //Cheking wheter the trasaction exist or not
//     pub fn transaction_exists( &self, user_id: u32, transaction_id: &str, ) -> bool { self.get_transaction(user_id, transaction_id).is_some() }




// }






