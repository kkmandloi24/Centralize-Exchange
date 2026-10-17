use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
// Import your Order struct from your command or market module
// use super::command::Order; 
use crate::models::Order;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExchangeSnapshot {
    pub sequence_id: u64,
    // Maps market names (e.g., "SolUsdc") to their respective order book snapshots
    pub markets: HashMap<String, MarketSnapshotData>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct MarketSnapshotData {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

impl ExchangeSnapshot {
    pub fn save(path: &str, snapshot: &ExchangeSnapshot) -> io::Result<()> {
        let data = serde_json::to_string(snapshot)?;
        let mut file = File::create(path)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    pub fn load(path: &str) -> io::Result<Option<ExchangeSnapshot>> {
        if !std::path::Path::new(path).exists() {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        let mut contents = String::new();  //create an variable which save values as string
        file.read_to_string(&mut contents)?;
        let snapshot: ExchangeSnapshot = serde_json::from_str(&contents)?;
        Ok(Some(snapshot))
    }
}