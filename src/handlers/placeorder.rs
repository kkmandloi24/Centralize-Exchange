
use crate::models::{SignupRequest,SignupResponse,ClientResponse, Order,Side,OrderType, OrderStatus};
use actix_web::{web, HttpMessage,HttpRequest, HttpResponse, Responder};
use crate::db::AppState;
use serde::{Serialize, Deserialize};

use crate::engine::{MarketPair,MatchingEngine,ExchangeServer};
use crate::wal::{Wal};

use tokio::sync::oneshot;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::BalanceCommand;


#[derive(Deserialize)]
pub struct OrderRequest {
    pub market: String,       // e.g., "SolUsdc"
    pub side: Side,           // Buy or Sell
    pub order_type: OrderType,// Limit or Market
    pub price: f64,
    pub quantity: f64,
}

// pub async fn place_order_handler(
//     state: web::Data<AppState>,
//     body: web::Json<OrderRequest>,
// ) -> impl Responder {
//     let user_id = 1; // In production, extract this from your verified JWT middleware

//     // 1. Determine base and quote assets based on the market string
//     // (Example: "SolUsdc" -> Base: "SOL", Quote: "USD")
//     let (base_asset, quote_asset) = match body.market.to_lowercase().as_str() {
//         "solusdc" => ("SOL", "USD"),
//         "btcusdc" => ("BTC", "USD"),
//         "ethusdc" => ("ETH", "USD"),
//         _ => return HttpResponse::BadRequest().json("Invalid or unsupported market pair"),
//     };

//     // 2. Calculate required asset and amount to lock
//     // BUY orders lock quote currency (USD) -> price * quantity
//     // SELL orders lock base currency (SOL/BTC/ETH) -> quantity
//     let (asset_to_lock, lock_amount) = match body.side {
//         Side::Buy => (quote_asset, body.price * body.quantity),
//         Side::Sell => (base_asset, body.quantity),
//     };

//     // 3. Request background balance worker to safely lock funds (Pre-trade Risk Check)
//     let (resp_tx, resp_rx) = oneshot::channel();
//     let _ = state.balance_tx.send(BalanceCommand::LockFunds {
//         user_id,
//         asset_identifier: asset_to_lock.to_string(),
//         amount: lock_amount,
//         resp: resp_tx,
//     }).await;

//     // Await response from the balance worker thread
//     match resp_rx.await {
//         Ok(Ok(_)) => { /* Funds successfully locked! Proceed to matching engine. */ }
//         Ok(Err(err_msg)) => return HttpResponse::BadRequest().json(err_msg),
//         Err(_) => return HttpResponse::InternalServerError().json("Balance worker communication failed"),
//     }

//     // 4. Construct the Order using your exact data model
//     let current_timestamp = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .unwrap()
//         .as_millis() as u64;

//     let order = Order {
//         id: 0, // Will be assigned deterministically by the matching engine
//         price: body.price,
//         quantity: body.quantity,
//         remaining_quantity: body.quantity, // Starts fully unfilled
//         order_type: body.order_type,
//         side: body.side,
//         status: OrderStatus::New,          // Initial status is New
//         timestamp: current_timestamp,      // FIFO price-time priority tie-breaker
//     };

//     // 5. Submit order to the Matching Engine and WAL
//     let market_pair = match body.market.to_lowercase().as_str() {
//         "solusdc" => MarketPair::SolUsdc,
//         "btcusdc" => MarketPair::BtcUsdc,
//         "ethusdc" => MarketPair::EthUsdc,
//         _ => unreachable!(),
//     };

//     //Now we get the Wal and server 
//     let wal_path = "exchange.wal";
//     let snapshot_file = "exchange.snapshot";

//     //if yet, not created the server and wal then initiate it

//     let (mut server,mut wal) = match ExchangeServer::new_with_snapshot_and_recovery("exchange.snapshot", "exchange.wal") {
//         Ok((server, wal)) => {
//             println!("Successfully recovered exchange state from disk.");
//             (server, wal)
//         }
//         Err(e) => {
//             println!("No existing snapshot/WAL found (or error: {}). Initializing a fresh exchange state...", e);
            
//             // Create brand new instances if recovery fails
//             let server = ExchangeServer::new(); // Or your default constructor
//             let wal = Wal::new("exchange.wal").unwrap();   // Initialize a new WAL file
            
//             (server, wal)
//         }
//     };

//     // Continue setting up your AppState and starting the server...
 




//     let (mut server, mut wal) = ExchangeServer::new_with_snapshot_and_recovery(snapshot_file, wal_path).unwrap();

//     let mut server = state.server.lock().unwrap();
//     let mut wal = state.wal.lock().unwrap();

//     if let Err(e) = server.place_order(market_pair, order, &mut wal) {
//         // SAFETY ROLLBACK: If the matching engine fails *after* locking funds,
//         // instantly unlock the funds back to the user's available balance!
//         let (rollback_tx, rollback_rx) = oneshot::channel();
//         let _ = state.balance_tx.send(BalanceCommand::UnlockFunds {
//             user_id,
//             asset_identifier: asset_to_lock.to_string(),
//             amount: lock_amount,
//             resp: rollback_tx,
//         }).await;
//         let _ = rollback_rx.await;

//         return HttpResponse::InternalServerError().json(format!("Matching engine error: {}", e));
//     }

//     // 6. Return success response
//     HttpResponse::Ok().json(serde_json::json!({
//         "status": "Success",
//         "message": "Order placed and funds locked successfully",
//         "user_id": user_id,
//         "market": body.market,
//         "side": body.side,
//         "price": body.price,
//         "quantity": body.quantity
//     }))
// }


pub async fn place_order_handler(
    state: web::Data<AppState>,
    body: web::Json<OrderRequest>,
    req: HttpRequest
) -> impl Responder {

    // 2. Extract the actual user_id stored by your JWT middleware (adjust type/key to match your middleware)
    // let user_id = match req.extensions().get::<u32>().cloned() {
    //     Some(id) => id,
    //     None => return HttpResponse::Unauthorized().json("Unauthorized user"),
    // };

    let user_id = match req.extensions().get::<u32>().copied() {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json("Unauthorized"),
    };
    // let user_id = 1; // Extract from JWT middleware in production

    // 1. Determine base and quote assets
    let (base_asset, quote_asset) = match body.market.to_lowercase().as_str() {
        "solusdc" => ("SOL", "USD"),
        "btcusdc" => ("BTC", "USD"),
        "ethusdc" => ("ETH", "USD"),
        _ => return HttpResponse::BadRequest().json("Invalid or unsupported market pair"),
    };

    // 2. Calculate required asset and amount to lock
    let (asset_to_lock, lock_amount) = match body.side {
        Side::Buy => (quote_asset, body.price * body.quantity),
        Side::Sell => (base_asset, body.quantity),
    };

    // 3. Request background balance worker to safely lock funds (Pre-trade Risk Check)
    let (resp_tx, resp_rx) = oneshot::channel();
    let _ = state.balance_tx.send(BalanceCommand::LockFunds {
        user_id,
        asset_identifier: asset_to_lock.to_string(),
        amount: lock_amount,
        resp: resp_tx,
    }).await;

    match resp_rx.await {
        Ok(Ok(_)) => { /* Funds successfully locked! */ }
        Ok(Err(err_msg)) => return HttpResponse::BadRequest().json(err_msg),
        Err(_) => return HttpResponse::InternalServerError().json("Balance worker communication failed"),
    }

    // 4. Construct the Order
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let order = Order {
        id: 0,
        price: body.price,
        quantity: body.quantity,
        remaining_quantity: body.quantity,
        order_type: body.order_type,
        side: body.side,
        status: OrderStatus::New,
        timestamp: current_timestamp,
    };

    let market_pair = match body.market.to_lowercase().as_str() {
        "solusdc" => MarketPair::SolUsdc,
        "btcusdc" => MarketPair::BtcUsdc,
        "ethusdc" => MarketPair::EthUsdc,
        _ => unreachable!(),
    };

    // 5. Access the shared server and WAL directly from AppState (No re-initializing!)
    let mut server = state.server.lock().unwrap();
    let mut wal = state.wal.lock().unwrap();

    if let Err(e) = server.place_order(market_pair, order, &mut wal) {
        // SAFETY ROLLBACK: Unlock funds if matching engine fails
        let (rollback_tx, rollback_rx) = oneshot::channel();
        let _ = state.balance_tx.send(BalanceCommand::UnlockFunds {
            user_id,
            asset_identifier: asset_to_lock.to_string(),
            amount: lock_amount,
            resp: rollback_tx,
        }).await;
        let _ = rollback_rx.await;

        return HttpResponse::InternalServerError().json(format!("Matching engine error: {}", e));
    }

    // 6. Return success response
    HttpResponse::Ok().json(serde_json::json!({
        "status": "Success",
        "message": "Order placed and funds locked successfully",
        "user_id": user_id,
        "market": body.market,
        "side": body.side,
        "price": body.price,
        "quantity": body.quantity
    }))
}