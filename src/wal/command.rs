use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

//DATA MODELS
use serde::{Serialize, Deserialize};
use crate::models::Order;
use crate::engine::MarketPair;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineCommand {
    PlaceOrder {
        market: MarketPair, // or MarketPair
        order: Order,   // Passes your entire Order struct!
    },
    CancelOrder {
        market: MarketPair,
        id: u64,
    },
   
    ModifyOrder {
        market: MarketPair,
        id: u64,
        new_price: f64,
        new_quantity: f64,
    }
}

pub struct Wal {
    writer: io::BufWriter<File>,
}
/////////////////////////////////////////////////




////////////////////////////////////////////


impl Wal {
    //CREATING FILE IN HARD DISC
    ///STEP 1). Initialize or open an existing WAL file in Hard Disk
    /// if file alredy exist then pth;URL :AsRef give refrence switch either it can string or &str or refrence of Path
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
            
        Ok(Self {
            writer: io::BufWriter::new(file),
        })
    }

    ///STEP 2). INSERTING A FILE IN THE BOTTOM OF FILE:  Append a command to disk and ensure it's flushed to hardware
    pub fn append(&mut self, command: &EngineCommand) -> io::Result<()> {
       //here we serialize command and write in the bottom of the created file

        //serialize the incoming orderCommand into serde_son string types
        let serialized = serde_json::to_string(command)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            //if error then wmap_err takes in error=e then handle it


        // 2. Write the JSON text, followed by a newline character (\n) so each order has its own line    
        self.writer.write_all(serialized.as_bytes())?;
        self.writer.write_all(b"\n")?; //write each comand in seprate lines
        
        // Flush buffer to disk to guarantee durability on crash
        self.writer.flush()?;

        // 4. Force the Operating System to physically write it to the hardware disk/SSD(before crash send all to disc/harddisc)
        self.writer.get_ref().sync_all()?; 
        
        Ok(())
    }


    //STEP 3: READ AND RECOVER FILE 
    /// Replay the WAL from disk to restore matching engine state on startup
    pub fn recover<P: AsRef<Path>>(path: P) -> io::Result<Vec<EngineCommand>> {
        if !path.as_ref().exists() {
            return Ok(Vec::new()); // No WAL exists yet (fresh start)
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        //creating empty vectore so we an save all command here after reading fro hard disc
        let mut commands = Vec::new();

        //now rading line by line the commands 
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // reading the line & making from line to command, and transforming in human readbale struct
            let command: EngineCommand = serde_json::from_str(&line)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            //then push in the vectore
            commands.push(command);
        }

        Ok(commands)
    }
}


//////////////

