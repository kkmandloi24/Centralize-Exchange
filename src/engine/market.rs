// src/engine/market.rs
use serde::{Serialize, Deserialize};
use crate::engine::{OrderBook};
use crate::models::Order;
use std::collections::HashMap;
use std::io;
use std::fs::File;
use std::io::{ Write, Read};
use crate::wal::{EngineCommand,Wal,ExchangeSnapshot,MarketSnapshotData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,Serialize, Deserialize)]
pub enum MarketPair {
    SolUsdc,
    BtcUsdc,
    EthUsdc,
}


pub struct MatchingEngine {
    pub order_book: OrderBook,
    pub next_sequence_number: u64, // <-- Order sequence lives here! we use it to give order id
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            next_sequence_number: 1, // Start sequence at 1
        }
    }

    // //calling cancel order
    // pub fn cancel_order(&mut self, order_id: u64) -> Option<Order> {
    //     self.order_book.cancel_order(order_id)
    // }



}

//Matches the order pair with the orderbook logic

pub struct ExchangeServer {
    pub engines: HashMap<MarketPair, MatchingEngine>,
}


//New impl ExchangeServer{} , coe with truncate or removed wall after snapshot
impl ExchangeServer {
//initiating server , as: 
pub fn new() -> Self {
        let mut engines = HashMap::new();

        // Initialize your 3 default trading pairs
        engines.insert(MarketPair::SolUsdc, MatchingEngine::new());
        engines.insert(MarketPair::BtcUsdc, MatchingEngine::new());
        engines.insert(MarketPair::EthUsdc, MatchingEngine::new());

        Self { engines }
    }



/////


    /// Create a new ExchangeServer with Snapshot loading + WAL recovery
    pub fn new_with_snapshot_and_recovery(snapshot_path: &str, wal_path: &str) -> io::Result<(Self, Wal)> {
        

        let mut server = Self::new();
        //created empty engine hashmap 
        // let mut engines = HashMap::new();

        // //then initiating 3 market seprately in the empty engine,so 3 markets exists separately
        // engines.insert(MarketPair::SolUsdc, MatchingEngine::new());
        // engines.insert(MarketPair::BtcUsdc, MatchingEngine::new());
        // engines.insert(MarketPair::EthUsdc, MatchingEngine::new());

        // //Server is the instance of the Exchangeserver here as we created below:
        // let mut server = Self { engines };


        let mut last_seq = 0;

        // 1. Try loading the latest snapshot first (instant baseline restore order,in the orderbook engine)
        if let Some(snapshot) = ExchangeSnapshot::load(snapshot_path)? {
            println!("Restored state from snapshot up to sequence ID: {}", snapshot.sequence_id);
            //after getting the order snapshots from load method of sbapshot we restore them in the engine
            server.restore_from_snapshot(snapshot);
            last_seq = server.get_max_sequence_id();  //max id of snapshot batch
        }

        // 2.from the wall Recover all past commands from the WAL file
        let all_commands = Wal::recover(wal_path)?;
        
        // 3. Filter WAL commands to only replay entries *newer* than the snapshot baseline
        let new_commands: Vec<_> = all_commands
            .into_iter()
            .filter(|cmd| {
                match cmd {
                    EngineCommand::PlaceOrder { order, .. } => order.id > last_seq,
                    EngineCommand::CancelOrder { id, .. } => *id > last_seq, 
                    EngineCommand::ModifyOrder { id, .. } => *id > last_seq, 
                }
            })
            .collect();

        println!("Replaying {} new events from WAL...", new_commands.len());

        // 4. Replay the remaining newer commands into the matching engines
        for command in new_commands {
            server.apply_restored_command(command);
        }

        // 5. Open WAL in append mode for future live writes
        let wal = Wal::new(wal_path)?;

        Ok((server, wal))
    }

    /// Create a snapshot and safely clear/truncate the old WAL file
    pub fn create_snapshot(&self, snapshot_path: &str, wal_path: &str) -> io::Result<()> {
        let mut markets_data = HashMap::new();

        //order id definer inside the snapshot
        let mut max_seq = 0;

        //in the Engines we have 2 things marketpair, and matching engine
        //we named market pair market, and matchingEngine as engine
        for (market, engine) in &self.engines {

            //get updated the max_seq
            if engine.next_sequence_number > max_seq {
                max_seq = engine.next_sequence_number;
            }

            
            let bids = engine.order_book.get_bids(); 
            let asks = engine.order_book.get_asks();

            let market_key = format!("{:?}", market); //tunr marketpair into strings

            //insert in the mepty hashmap the entie engine ingredients
            markets_data.insert(
                market_key, 
                MarketSnapshotData { bids, asks },
            );
        }

        //making ExchangeSnapShot form engine
        let snapshot = ExchangeSnapshot {
            sequence_id: max_seq,
            markets: markets_data,
        };

        //creating snapshot
        // 1. Save snapshot first (safety check)
        ExchangeSnapshot::save(snapshot_path, &snapshot)?;
        println!("Snapshot successfully created at sequence ID: {}", max_seq);

        // 2. Clear/truncate the old WAL file since everything is now baked into the snapshot
        let _fresh_wal = std::fs::File::create(wal_path)?;
        println!("Old WAL data cleared successfully. Starting fresh.");

        Ok(())
    }

    //We take data from snapshot to RAM. to restore the engine (market + orderbooks)

    /// Helper to restore engine state from a loaded snapshot
    fn restore_from_snapshot(&mut self, snapshot: ExchangeSnapshot) {
        for (market_str, market_data) in snapshot.markets {
            let market = match market_str.as_str() {
                "SolUsdc" => MarketPair::SolUsdc,
                "BtcUsdc" => MarketPair::BtcUsdc,
                "EthUsdc" => MarketPair::EthUsdc,
                _ => continue,
            };

            //we get engine rspect to the market
            //then update: next_sequence_number
            //

            if let Some(engine) = self.engines.get_mut(&market) {
                for order in market_data.bids {
                    //why we update next_sequnece_number= becuase when system starts it starts from 1, but in the snapshot we have last order id=10 restored
                    //when the order.id comes it is 10>=1 then it reupdate the next_sequence_number= 10+1
                    if order.id >= engine.next_sequence_number {
                        engine.next_sequence_number = order.id + 1;
                    }
                    let _ = engine.order_book.add_order(order);
                }
                for order in market_data.asks {
                    if order.id >= engine.next_sequence_number {
                        engine.next_sequence_number = order.id + 1;
                    }
                    let _ = engine.order_book.add_order(order);
                }
            }
        }
    }


    /// Utility to find the highest active sequence number across all engines
    /// we iterate on the engine values
    fn get_max_sequence_id(&self) -> u64 {
        self.engines.values()
            .map(|engine| engine.next_sequence_number)
            .max()
            .unwrap_or(0)
    }

    //RESUMING ORDER COMMANDS FROM WAL AFTER THE SNAPSHOT (GAP ORDER COMMANDS),apply_restored_command
    /// Internal helper to rebuild state during startup without rewriting to WAL
    fn apply_restored_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::PlaceOrder { market, order } => {
                if let Some(engine) = self.engines.get_mut(&market) {
                    if order.id >= engine.next_sequence_number {
                        engine.next_sequence_number = order.id + 1;
                    }
                    let _trades = engine.order_book.add_order(order);
                }
            }
            EngineCommand::CancelOrder { market, id } => {
                if let Some(engine) = self.engines.get_mut(&market) {
                    let _ = engine.order_book.cancel_order(id);
                }
            }
            EngineCommand::ModifyOrder { market, id, new_price, new_quantity } => {
                if let Some(engine) = self.engines.get_mut(&market) {
                    let _ = engine.order_book.modify_order(id, new_price, new_quantity);
                }
            }
        }
    }

    //Live order placement: Assigns ID, logs to WAL first, then matches in engine
    pub fn place_order(&mut self, market: MarketPair, mut order: Order, wal: &mut Wal) -> io::Result<()> {
        let mut assigned_id= 0;

        {
        let engine=match self.engines.get_mut(&market){
            Some(eng)=>eng,
            None=>return Ok(()),
        };

        
        // if let Some(engine) = self.engines.get_mut(&market) {
            order.id = engine.next_sequence_number;
            engine.next_sequence_number += 1;

        // Save the ID into a variable *before* `order` gets moved!( we took id from order and saved it in another variable)
        assigned_id = order.id;
            

            let command = EngineCommand::PlaceOrder {
                market,
                order: order.clone(),
            };

            wal.append(&command)?;

            let trades = engine.order_book.add_order(order);
            println!("Market {:?} processed order. Trades: {:?}", market, trades);

           
        }

        if assigned_id%10_000==0{
             let wal_path = "exchange.wal";
             let snapshot_file = "exchange.snapshot";

            println!("Threshold reached! Triggering automatic snapshot...");
            self.create_snapshot(snapshot_file, wal_path)?;
        }

        
        Ok(())
    }


    /// Live order cancellation with WAL logging
    pub fn cancel_order(&mut self, market: MarketPair, id: u64, wal: &mut Wal) -> io::Result<()> {
        if let Some(engine) = self.engines.get_mut(&market) {
            let command = EngineCommand::CancelOrder { market, id };
            wal.append(&command)?;
            let cancelled = engine.order_book.cancel_order(id);
            println!("Market {:?} cancelled order ID {}: Successfully removed = {:?}", market, id, cancelled.is_some());
        }
        Ok(())
    }



    /// Live order modification with WAL logging
    pub fn modify_order(&mut self, market: MarketPair, id: u64, new_price: f64, new_quantity: f64, wal: &mut Wal) -> io::Result<()> {
        if let Some(engine) = self.engines.get_mut(&market) {
            let command = EngineCommand::ModifyOrder { market, id, new_price, new_quantity };
            wal.append(&command)?;
            let modified = engine.order_book.modify_order(id, new_price, new_quantity);
            println!("Market {:?} modified order ID {}: Successfully modified = {:?}", market, id, modified.is_some());
        }
        Ok(())
    }
}









//EXCHANGESERVER IMPL BLock without wal deleting code after the snapshot

//new impl block on the ExchangeServer, with considering the wal (wrte ahead logging- means save order commands before execute)


////// impl ExchangeServer code block with WAL and SNAPSHOT features///////////




// impl ExchangeServer {
//     /// Create a new ExchangeServer with Snapshot loading + WAL recovery
//     pub fn new_with_snapshot_and_recovery(snapshot_path: &str, wal_path: &str) -> io::Result<(Self, Wal)> {
        
//         //created empty engine hashmap 
//         let mut engines = HashMap::new();

//         //inserting key and value
//         engines.insert(MarketPair::SolUsdc, MatchingEngine::new());
//         engines.insert(MarketPair::BtcUsdc, MatchingEngine::new());
//         engines.insert(MarketPair::EthUsdc, MatchingEngine::new());

//         let mut server = Self { engines };
//         let mut last_seq = 0;

//         // 1. Try loading the latest snapshot first (instant baseline restore)
//         if let Some(snapshot) = ExchangeSnapshot::load(snapshot_path)? {
//             println!("Restored state from snapshot up to sequence ID: {}", snapshot.sequence_id);
//             server.restore_from_snapshot(snapshot);
//             last_seq = server.get_max_sequence_id();  //max id of snapshot batch
//         }

//         // 2. Recover all past commands from the WAL file
//         let all_commands = Wal::recover(wal_path)?;
        
//         // 3. Filter WAL commands to only replay entries *newer* than the snapshot baseline
//         let new_commands: Vec<_> = all_commands
//             .into_iter()
//             .filter(|cmd| {
//                 match cmd {
//                     EngineCommand::PlaceOrder { order, .. } => order.id > last_seq,
//                     EngineCommand::CancelOrder { id, .. } => *id > last_seq, // Optional: filter old cancels if needed
//                     EngineCommand::ModifyOrder { id, .. } => *id > last_seq, // Optional: filter old modifies if needed
//                 }
//             })
//             .collect();

//         println!("Replaying {} new events from WAL...", new_commands.len());

//         // 4. Replay the remaining newer commands into the matching engines
//         for command in new_commands {
//             server.apply_restored_command(command);
//         }

//         // 5. Open WAL in append mode for future live writes
//         let wal = Wal::new(wal_path)?;

//         Ok((server, wal))
//     }

//     pub fn create_snapshot(&self, path: &str) -> io::Result<()> {
//         let mut markets_data = HashMap::new();
//         let mut max_seq = 0;

//         for (market, engine) in &self.engines {
//             if engine.next_sequence_number > max_seq {
//                 max_seq = engine.next_sequence_number;
//             }

//             let bids = engine.order_book.get_bids(); 
//             let asks = engine.order_book.get_asks();

//             let market_key = format!("{:?}", market);
//             markets_data.insert(
//                 market_key, 
//                 MarketSnapshotData { bids, asks },
//             );
//         }

//         let snapshot = ExchangeSnapshot {
//             sequence_id: max_seq,
//             markets: markets_data,
//         };

//         ExchangeSnapshot::save(path, &snapshot)?;
//         println!("Snapshot successfully created at sequence ID: {}", max_seq);
//         Ok(())
//     }

//     /// Helper to restore engine state from a loaded snapshot
//     fn restore_from_snapshot(&mut self, snapshot: ExchangeSnapshot) {
//         for (market_str, market_data) in snapshot.markets {
//             let market = match market_str.as_str() {
//                 "SolUsdc" => MarketPair::SolUsdc,
//                 "BtcUsdc" => MarketPair::BtcUsdc,
//                 "EthUsdc" => MarketPair::EthUsdc,
//                 _ => continue,
//             };

//             if let Some(engine) = self.engines.get_mut(&market) {
//                 for order in market_data.bids {
//                     if order.id >= engine.next_sequence_number {
//                         engine.next_sequence_number = order.id + 1;
//                     }
//                     let _ = engine.order_book.add_order(order);
//                 }
//                 for order in market_data.asks {
//                     if order.id >= engine.next_sequence_number {
//                         engine.next_sequence_number = order.id + 1;
//                     }
//                     let _ = engine.order_book.add_order(order);
//                 }
//             }
//         }
//     }

//     /// Utility to find the highest active sequence number across all engines
//     fn get_max_sequence_id(&self) -> u64 {
//         self.engines.values()
//             .map(|engine| engine.next_sequence_number)
//             .max()
//             .unwrap_or(0)
//     }

//     /// Internal helper to rebuild state during startup without rewriting to WAL
//     fn apply_restored_command(&mut self, command: EngineCommand) {
//         match command {
//             EngineCommand::PlaceOrder { market, order } => {
//                 if let Some(engine) = self.engines.get_mut(&market) {
//                     if order.id >= engine.next_sequence_number {
//                         engine.next_sequence_number = order.id + 1;
//                     }
//                     let _trades = engine.order_book.add_order(order);
//                 }
//             }
//             EngineCommand::CancelOrder { market, id } => {
//                 if let Some(engine) = self.engines.get_mut(&market) {
//                     let _ = engine.order_book.cancel_order(id);
//                 }
//             }
//             // --- ADDED: Handle ModifyOrder during WAL recovery ---
//             EngineCommand::ModifyOrder { market, id, new_price, new_quantity } => {
//                 if let Some(engine) = self.engines.get_mut(&market) {
//                     let _ = engine.order_book.modify_order(id, new_price, new_quantity);
//                 }
//             }
//         }
//     }

//     /// Live order placement: Assigns ID, logs to WAL first, then matches in engine
//     pub fn place_order(&mut self, market: MarketPair, mut order: Order, wal: &mut Wal) -> io::Result<()> {
//         if let Some(engine) = self.engines.get_mut(&market) {
//             order.id = engine.next_sequence_number;
//             engine.next_sequence_number += 1;

//             let command = EngineCommand::PlaceOrder {
//                 market,
//                 order: order.clone(),
//             };

//             wal.append(&command)?;
//             let trades = engine.order_book.add_order(order);
//             println!("Market {:?} processed order. Trades: {:?}", market, trades);
//         }
//         Ok(())
//     }

//     /// --- ADDED: Live order cancellation with WAL logging ---
//     pub fn cancel_order(&mut self, market: MarketPair, id: u64, wal: &mut Wal) -> io::Result<()> {
//         if let Some(engine) = self.engines.get_mut(&market) {
//             let command = EngineCommand::CancelOrder { market, id };
//             wal.append(&command)?;
//             let cancelled = engine.order_book.cancel_order(id);
//             println!("Market {:?} cancelled order ID {}: Successfully removed = {:?}", market, id, cancelled.is_some());
//         }
//         Ok(())
//     }

//     /// --- ADDED: Live order modification with WAL logging ---
//     pub fn modify_order(&mut self, market: MarketPair, id: u64, new_price: f64, new_quantity: f64, wal: &mut Wal) -> io::Result<()> {
//         if let Some(engine) = self.engines.get_mut(&market) {
//             let command = EngineCommand::ModifyOrder { market, id, new_price, new_quantity };
//             wal.append(&command)?;
//             let modified = engine.order_book.modify_order(id, new_price, new_quantity);
//             println!("Market {:?} modified order ID {}: Successfully modified = {:?}", market, id, modified.is_some());
//         }
//         Ok(())
//     }
// }



