// src/engine/orderbook.rs

// src/engine/orderbook.rs

use crate::models::{Order, Side, OrderType, OrderStatus, Trade};

#[derive(Debug,)]
pub struct OrderBook {
    bids: Vec<Order>, // Buy orders (sorted descending by price)
    asks: Vec<Order>, // Sell orders (sorted ascending by price)
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    // --- ADD THESE TWO GETTERS ---which gives clone

    pub fn get_bids(&self) -> Vec<Order> {
        self.bids.clone()
    }

    pub fn get_asks(&self) -> Vec<Order> {
        self.asks.clone()
    }

    /// Core matching engine algorithm implementing Price-Time Priority
    pub fn add_order(&mut self, mut order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        match order.side {
            Side::Buy => {
                // A buy order matches against existing ASKS (sell orders)
                while order.remaining_quantity > 0.0 && !self.asks.is_empty() {
                    // Sort asks ascending: lowest price first, then earliest timestamp (Time Priority)
                    self.asks.sort_by(|a, b| {
                        a.price.partial_cmp(&b.price).unwrap()
                            .then_with(|| a.timestamp.cmp(&b.timestamp)).then_with(|| a.id.cmp(&b.id))
                    });

                    let best_ask = &mut self.asks[0];

                    // If it's a Limit order and the buyer's price is lower than the best ask, stop matching
                    // Market orders bypass this price check completely.
                    if order.order_type == OrderType::Limit && order.price < best_ask.price {
                        break;
                    }

                    // Calculate execution quantity
                    let trade_qty = order.remaining_quantity.min(best_ask.remaining_quantity);

                    // Record the trade (Executes at the resting order's price!)
                    trades.push(Trade {
                        price: best_ask.price,
                        quantity: trade_qty,
                        buyer_order_id: order.id,
                        seller_order_id: best_ask.id,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    });

                    // Update remaining quantities
                    order.remaining_quantity -= trade_qty;
                    best_ask.remaining_quantity -= trade_qty;

                    // Update order statuses
                    order.status = if order.remaining_quantity == 0.0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    best_ask.status = if best_ask.remaining_quantity == 0.0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    // Remove fully filled ask from the order book
                    if best_ask.status == OrderStatus::Filled {
                        self.asks.remove(0);
                    }
                }

                // Post-matching handling for Buy orders
                if order.remaining_quantity > 0.0 {
                    match order.order_type {
                        OrderType::Limit => {
                            // Unfilled Limit orders rest on the bids book
                            order.status = if order.status == OrderStatus::PartiallyFilled {
                                OrderStatus::PartiallyFilled
                            } else {
                                OrderStatus::New
                            };
                            self.bids.push(order);
                        }
                        OrderType::Market => {
                            // Unfilled Market orders (due to lack of liquidity) are cancelled
                            order.status = OrderStatus::Cancelled;
                        }
                    }
                }
            }
            Side::Sell => {
                // A sell order matches against existing BIDS (buy orders)
                while order.remaining_quantity > 0.0 && !self.bids.is_empty() {
                    // Sort bids descending: highest price first, then earliest timestamp
                    self.bids.sort_by(|a, b| {
                        b.price.partial_cmp(&a.price).unwrap()
                            .then_with(|| a.timestamp.cmp(&b.timestamp)).then_with(|| b.id.cmp(&a.id))
                    });

                    let best_bid = &mut self.bids[0];

                    // If it's a Limit order and the seller's price is higher than the best bid, stop matching
                    // Market orders bypass this price check.
                    if order.order_type == OrderType::Limit && order.price > best_bid.price {
                        break;
                    }

                    let trade_qty = order.remaining_quantity.min(best_bid.remaining_quantity);

                    trades.push(Trade {
                        price: best_bid.price,
                        quantity: trade_qty,
                        buyer_order_id: best_bid.id,
                        seller_order_id: order.id,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    });

                    order.remaining_quantity -= trade_qty;
                    best_bid.remaining_quantity -= trade_qty;

                    order.status = if order.remaining_quantity == 0.0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    best_bid.status = if best_bid.remaining_quantity == 0.0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    if best_bid.status == OrderStatus::Filled {
                        self.bids.remove(0);
                    }
                }

                // Post-matching handling for Sell orders
                if order.remaining_quantity > 0.0 {
                    match order.order_type {
                        OrderType::Limit => {
                            order.status = if order.status == OrderStatus::PartiallyFilled {
                                OrderStatus::PartiallyFilled
                            } else {
                                OrderStatus::New
                            };
                            self.asks.push(order);
                        }
                        OrderType::Market => {
                            order.status = OrderStatus::Cancelled;
                        }
                    }
                }
            }
        }

        trades
    }


//MODIFY ORDER
pub fn modify_order(&mut self, order_id: u64, new_price: f64, new_quantity: f64) -> Option<Order> {
        // Step 1: Find and remove the existing order
        let mut order = self.cancel_order(order_id)?;

        // Step 2: Update its fields
        order.price = new_price;
        order.quantity = new_quantity;
        order.remaining_quantity = new_quantity; 

        // Step 3: Push back and re-sort with ID tie-breaker
        match order.side {
            Side::Buy => {
                self.bids.push(order.clone());
                // Bids: Highest price first -> Earliest timestamp -> Lowest order ID (Sequence ID)
                self.bids.sort_by(|a, b| {
                    b.price.partial_cmp(&a.price).unwrap()
                        .then_with(|| a.timestamp.cmp(&b.timestamp))
                        .then_with(|| a.id.cmp(&b.id)) // <--- Sequence ID tie-breaker
                });
            }
            Side::Sell => {
                self.asks.push(order.clone());
                // Asks: Lowest price first -> Earliest timestamp -> Lowest order ID (Sequence ID)
                self.asks.sort_by(|a, b| {
                    a.price.partial_cmp(&b.price).unwrap()
                        .then_with(|| a.timestamp.cmp(&b.timestamp))
                        .then_with(|| a.id.cmp(&b.id)) // <--- Sequence ID tie-breaker
                });
            }
        }

        Some(order)
    }



    //CANCEL ORDER

    /// Find and remove an order from either bids or asks by its ID
    pub fn cancel_order(&mut self, order_id: u64) -> Option<Order> {
        // 1. Check if it's in the bids vector
        if let Some(index) = self.bids.iter().position(|o| o.id == order_id) {
            return Some(self.bids.remove(index));
        }

        // 2. Check if it's in the asks vector
        if let Some(index) = self.asks.iter().position(|o| o.id == order_id) {
            return Some(self.asks.remove(index));
        }

        // 3. Not found anywhere
        None
    }
}

//////////////////////////////////////////////

// pub fn add_order(&mut self, mut order:Order)->Vec<Trade>{

//     //1. CREATED TRADES VECTOR INITAILIZATION
//     let mut trades= Vec::new();


//     // 2. SELECTING INCOMING ORDER SIDE 
//    match order.side{
// Side::Buy=>{


//    //a. if its buy we sort the sellers(ask) order on orderbook as ascedning order so buyer will see lowest price first
//    while order.remaining_quantity>0.0 && !self.asks.is_empty(){

//     //3. CRETING ASCENDING ORDER, of seller's order on orderbook
//     self.asks.price.sort_by(|a,b|{
//         a.price.partial_cmp(&b.price).then_with(||a.time_stamp.partial_cmp(&b.time_stamp));
//     });

//     //4. GET LOWEST PRICE THAT WILL EXECUTE FIRST= is best price buyer, now we are sure the lowest price will be at the position 0 in vector ask
//     let best_asks= &mut self.asks[0];

//     //5.IF INCOMING ORDER IS LIMIT AND PRICE OF ORDER IS HIGHER THEN THE BEST ASK PRICE ORDER THEN SKIP the loop
//     // if Order is limit and the price of the order is low then lowes sales price, hen not run loop on it
//     if order.order_type==OrderType::Limit && order.price<best_ask.price{
// break;
//     }
        
// //if ordr is even limit but its price is below or equal to the ask price we run loop and execute it
// //if order is marketprice then we run loop and check its price to find the best ask price order

// //in all other order we have to execute the order even it 
// //so first we define the qnty

// //if order will execute then the quntity will
// //dono me jo minimum hogi vo lega min function


// //6. DEFINING THE TRADE QUNTITI USIN min() function
// let trade_qty= order.remaining_quantity.min(&best_ask.remaining_qauntity);


// //7. PUSHING TRADES ORDER ENTRIES IN THE TRADES VECTOR 
// trades.push(Trade{
//     price:best_asks.price, // Executes at the resting order's price!
//     quantity:trades_qnty,
//     buyer_order_id: order.id,
//     seller_order_id:best_asks.id,
//     timestamp: std::time::systemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().duration.as_millis() as u64,

// })

// //8. DEFINING REMAINING QUANTITY OF INCOIMG ORDER & RESTING ORDER

// //jo quntity will traded(executed) will be decreased form both seler and buyers order
// //after trade updated buyer and seller remaining quantity
// order.remaining_quantity- =trades_qty;
// best_asks.remaining_quantiy-=trades_qty


// //9. DEFINING THE STATUS on iNCOMING & RESTING ORDER
// //now we define the incoming order status and resting order(ask order) status
// order.status= if order.remaining_quantity==00{
//     OrderStatus::Filled
// }else{
//     OrderStatus::PartiallyFilled
// }

// best_asks.status= if best_asks.remaining_quantity==0{
//     OrderStatus::Filled
// }else{
//     OrderStatus::PartiallyFilled
// }

// //10. REMOVING THE FULLY FILLED RESTING ORDER
// if best_asks.status=Filled{
//     self.asks.remove(0);  //otherwise it will show 0 quntity with the orders
// }


// //POST MATCHING (IF ORDERS AFTER EXECUTION REMAINED SOME QUANTITY )
// //here 2 types order could be here- LiMIT MARKET, first we check if rder have remianing quantity >0.0 then handle on these types order 
// //pos matching logic

// if order.remaining_quantity>0.0{
//     match order.order_type{
//      OrderType::Limit=>{
//      order.status== if order.status==OrderStatus::PartiallyFilled{
//         OrderStatus::PartiallyFilled
//      } else{
//         OrderStatus::New
//      } 
//      //then again we push the same order on the orderbook
//      bids.push(order);
//      }

//      //market order either filled or cancelled (not sitson the orderbook so if not matched we have tocancelled them)
//      OrderType::Market=>{
//         order.status= OrderStatus::Cancelled

//         //market orders not required to push again, either they execute or got cancelled

//      }

//     }

// Side::Sell=>{
//     //here on the sell side the order(incoming order , while resting order will be bid side)
//     while self.order.reamining_quantity>0.0 && !self.bids.is_empty(){
        
//         //place all orders in descending order on the orderbook( because seller want to see hie price for his order)
//         self.bids.price.sort_by(|a,b|{
//             b.price.partially_cmp(&a.price).unramp().then_with(|| b.timestamp.pratially_cmp(&a.timestamp))

//         })

//         //now we find the best price order to be for seller
//         let best_bids= &mut self.bids[0]   //we picked the best price

        
//         //We skip the incoming(seller order) limit orders which price is higher then the resting  buyers order 
//         if order.price>best_bids.price && order.order_type==OrderType::Limit{
//             break;
//         }

//         //now below or equal the price of seller incoming orders (even limit or market order) then the best_bids.price will execute
//         //we consider thos simply like incoming order

//         let trade_qnty= order.remaining_quantity.min(best_bids.remaining_quantity);

//         //pushed traded order in the trades vector/db
//         trades.push(Trade{
//          price:best_bids.price,
//          quantity:trades_qnty,
//          seller_order_id:order.id,
//          buyer_order_id:best_bids.id,
//          timestamp:std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().duration.as_millis() as u64;

//         })

//         //what is the traded quntity we substract it from incoming order'r remaining quntity that order is traded
//         order.remaining_qunatit=order.remaining_qunatit-trades_qnty;  
//         //short hand:  order.remaining_qunatit-=trades_qnty;

//         best_bids.remaining_quantity= best_bids.remaining_quantity-trades_qnty;

//         //now we work on the order status:

//         order.status= if order.remaining_quantity==0.0{
//             OrderStatus::Filled;
//         }else{
//             OrderStatus::PartiallyFilled;
//         }

//         best_bids.status= if best_bids.remaining_quantity==0.0{
//             OrderStatus::Filled;
//         }else{
//             OrderStatus::PartiallyFilled;
//         }

//         if best_bids.status==OrderStatus::Filled{
//             self.bids.remove(0); //so we removed the bids indexed 0 element which was the best price(hihest) one to buy from seller
//         }

//         //after execution of order if incoming order has some remiaing quantty then we have to place it on the its side
//         order.remaining_quantity>0.0{
//             match order.order_type{
//                 OrderType::Limit=>{
//                     order.status= if order.status==OrderStatus::PartiallyFilled{
//                         OrderStatus::PartiallyFilled

//                     } else{New

//                     }
//                     //if the order is partiallfilled or the order is the new we have to pushed it again on the asks side
//                    self.asks.push(order);

//                 }
//                 OrderType::Market=>{
//                     order.status=OrderStatus::Cancelled;

//                 }
//             }

//         } 







//     }
// }

// }












    


//     }//



//    }//

//    trades

//  }//


//    }//









////////////


// src/engine/orderbook.rs

// use crate::models::{Order, Side, OrderStatus, Trade};

// pub struct OrderBook {
//     bids: Vec<Order>, // Buy orders (sorted descending by price)
//     asks: Vec<Order>, // Sell orders (sorted ascending by price)
// }

// impl OrderBook {
//     pub fn new() -> Self {
//         Self {
//             bids: Vec::new(),
//             asks: Vec::new(),
//         }
//     }

//     /// Core matching engine algorithm using Price-Time Priority
//     pub fn add_order(&mut self, mut order: Order) -> Vec<Trade> {
//         let mut trades = Vec::new();

//         match order.side {
//             Side::Buy => {
//                 // A buy order matches against existing ASKS (sell orders)
//                 while order.remaining_quantity > 0.0 && !self.asks.is_empty() {
//                     // Sort asks ascending: lowest price first, then earliest timestamp
//                     self.asks.sort_by(|a, b| {
//                         a.price.partial_cmp(&b.price).unwrap()
//                             .then_with(|| a.timestamp.cmp(&b.timestamp))
//                     });

//                     let best_ask = &mut self.asks[0];

//                     // If buyer's limit price is lower than the best ask, stop matching
//                     if order.price < best_ask.price && order.order_type == crate::models::OrderType::Limit {
//                         break;
//                     }

//                     // Calculate execution quantity
//                     let trade_qty = order.remaining_quantity.min(best_ask.remaining_quantity);

//                     // Record the trade (executes at the resting order's price)
//                     trades.push(Trade {
//                         price: best_ask.price,
//                         quantity: trade_qty,
//                         buyer_order_id: order.id,
//                         seller_order_id: best_ask.id,
//                         timestamp: std::time::SystemTime::now()
//                             .duration_since(std::time::UNIX_EPOCH)
//                             .unwrap()
//                             .as_millis() as u64,
//                     });

//                     // Update remaining quantities
//                     order.remaining_quantity -= trade_qty;
//                     best_ask.remaining_quantity -= trade_qty;

//                     // Update order statuses
//                     order.status = if order.remaining_quantity == 0.0 {
//                         OrderStatus::Filled
//                     } else {
//                         OrderStatus::PartiallyFilled
//                     };

//                     best_ask.status = if best_ask.remaining_quantity == 0.0 {
//                         OrderStatus::Filled
//                     } else {
//                         OrderStatus::PartiallyFilled
//                     };

//                     // Remove fully filled ask from the order book
//                     if best_ask.status == OrderStatus::Filled {
//                         self.asks.remove(0);
//                     }
//                 }

//                 // If buy order still has quantity left, rest it on the bids book
//                 if order.remaining_quantity > 0.0 && order.order_type == crate::models::OrderType::Limit {
//                     self.bids.push(order);
//                 }
//             }
//             Side::Sell => {
//                 // A sell order matches against existing BIDS (buy orders)
//                 while order.remaining_quantity > 0.0 && !self.bids.is_empty() {
//                     // Sort bids descending: highest price first, then earliest timestamp
//                     self.bids.sort_by(|a, b| {
//                         b.price.partial_cmp(&a.price).unwrap()
//                             .then_with(|| a.timestamp.cmp(&b.timestamp))
//                     });

//                     let best_bid = &mut self.bids[0];

//                     // If seller's limit price is higher than the best bid, stop matching
//                     if order.price > best_bid.price && order.order_type == crate::models::OrderType::Limit {
//                         break;
//                     }

//                     let trade_qty = order.remaining_quantity.min(best_bid.remaining_quantity);

//                     trades.push(Trade {
//                         price: best_bid.price,
//                         quantity: trade_qty,
//                         buyer_order_id: best_bid.id,
//                         seller_order_id: order.id,
//                         timestamp: std::time::SystemTime::now()
//                             .duration_since(std::time::UNIX_EPOCH)
//                             .unwrap()
//                             .as_millis() as u64,
//                     });

//                     order.remaining_quantity -= trade_qty;
//                     best_bid.remaining_quantity -= trade_qty;

//                     order.status = if order.remaining_quantity == 0.0 {
//                         OrderStatus::Filled
//                     } else {
//                         OrderStatus::PartiallyFilled
//                     };

//                     best_bid.status = if best_bid.remaining_quantity == 0.0 {
//                         OrderStatus::Filled
//                     } else {
//                         OrderStatus::PartiallyFilled
//                     };

//                     if best_bid.status == OrderStatus::Filled {
//                         self.bids.remove(0);
//                     }
//                 }

//                 // If sell order still has quantity left, rest it on the asks book
//                 if order.remaining_quantity > 0.0 && order.order_type == crate::models::OrderType::Limit {
//                     self.asks.push(order);
//                 }
//             }
//         }

//         trades
//     }
// }




//////////////////////////////////////////////


// use crate::models::{Order, Side, OrderStatus, Trade};
// use std::collections::VecDeque;

// pub struct OrderBook {
//     // For a simple, clean portfolio MVP, we can store orders in vectors or queues.
//     // In production, matching engines use B-Tree maps or specialized price-level indexes.
//     bids: Vec<Order>, // Buy orders
//     asks: Vec<Order>, // Sell orders
// }

// impl OrderBook {
//     pub fn new() -> Self {
//         Self {
//             bids: Vec::new(),
//             asks: Vec::new(),
//         }
//     }

//     /// Core function to add an order and attempt matching
//     pub fn add_order(&mut self, mut order: Order) -> Vec<Trade> {
//         //we created a empty vector for keeping the filled orders
//         let mut trades = Vec::new();

//         match order.side {
//             Side::Buy => {
//                 // A buy order matches against existing ASKS (sell orders)
//                 // Condition: Buy price >= Ask price
//                 while order.remaining_quantity > 0.0 && !self.asks.is_empty() {
//                     // Sort asks ascending so the lowest price is always at index 0
//                     self.asks.sort_by(|a, b| {
//                         a.price.partial_cmp(&b.price).unwrap()
//                             .then_with(|| a.timestamp.cmp(&b.timestamp)) // Time priority tie-breaker
//                     });

//                     let best_ask = &mut self.asks[0];

//                     // If the buyer's price is lower than the best ask, no match can happen
//                     if order.price < best_ask.price && order.order_type == crate::models::OrderType::Limit {
//                         break;
//                     }

//                     // Determine trade quantity
//                     let trade_qty = order.remaining_quantity.min(best_ask.remaining_quantity);

//                     // Create the trade record
//                     trades.push(Trade {
//                         price: best_ask.price, // Execution happens at the resting order's price
//                         quantity: trade_qty,
//                         buyer_order_id: order.id,
//                         seller_order_id: best_ask.id,
//                         timestamp: std::time::SystemTime::now()
//                             .duration_since(std::time::UNIX_EPOCH)
//                             .unwrap()
//                             .as_millis() as u64,
//                     });

//                     // Update remaining quantities
//                     order.remaining_quantity -= trade_qty;
//                     best_ask.remaining_quantity -= trade_qty;

//                     // Update order statuses
//                     if order.remaining_quantity == 0.0 {
//                         order.status = OrderStatus::Filled;
//                     } else {
//                         order.status = OrderStatus::PartiallyFilled;
//                     }

//                     if best_ask.remaining_quantity == 0.0 {
//                         best_ask.status = OrderStatus::Filled;
//                     } else {
//                         best_ask.status = OrderStatus::PartiallyFilled;
//                     }

//                     // Remove fully filled asks from the book
//                     if best_ask.status == OrderStatus::Filled {
//                         self.asks.remove(0);
//                     }
//                 }

//                 // If the buy order still has remaining quantity, push it to the bids book
//                 if order.remaining_quantity > 0.0 && order.order_type == crate::models::OrderType::Limit {
//                     self.bids.push(order);
//                 }
//             }


//             Side::Sell => {
//                 // A sell order matches against existing BIDS (buy orders)
//                 // Condition: Sell price <= Bid price
//                 while order.remaining_quantity > 0.0 && !self.bids.is_empty() {
//                     // Sort bids descending so the highest price is always at index 0
//                     self.bids.sort_by(|a, b| {
//                         b.price.partial_cmp(&a.price).unwrap()
//                             .then_with(|| a.timestamp.cmp(&b.timestamp)) // Time priority tie-breaker
//                     });

//                     let best_bid = &mut self.bids[0];

//                     // If the seller's price is higher than the best bid, no match can happen
//                     if order.price > best_bid.price && order.order_type == crate::models::OrderType::Limit {
//                         break;
//                     }

//                     let trade_qty = order.remaining_quantity.min(best_bid.remaining_quantity);

//                     trades.push(Trade {
//                         price: best_bid.price,
//                         quantity: trade_qty,
//                         buyer_order_id: best_bid.id,
//                         seller_order_id: order.id,
//                         timestamp: std::time::SystemTime::now()
//                             .duration_since(std::time::UNIX_EPOCH)
//                             .unwrap()
//                             .as_millis() as u64,
//                     });

//                     order.remaining_quantity -= trade_qty;
//                     best_bid.remaining_quantity -= trade_qty;

//                     if order.remaining_quantity == 0.0 {
//                         order.status = OrderStatus::Filled;
//                     } else {
//                         order.status = OrderStatus::PartiallyFilled;
//                     }

//                     if best_bid.remaining_quantity == 0.0 {
//                         best_bid.status = OrderStatus::Filled;
//                     } else {
//                         best_bid.status = OrderStatus::PartiallyFilled;
//                     }

//                     if best_bid.status == OrderStatus::Filled {
//                         self.bids.remove(0);
//                     }
//                 }

//                 if order.remaining_quantity > 0.0 && order.order_type == crate::models::OrderType::Limit {
//                     self.asks.push(order);
//                 }
//             }
//         }

//         trades
//     }
// }

// /////////////////////////////////////////////////////

// // src/main.rs
// mod models;
// mod engine;

// use models::{Order, Side, OrderType, OrderStatus};
// use engine::OrderBook;

// fn main() {
//     let mut orderbook = OrderBook::new();

//     // 1. Place a Sell Order (Ask) at $100 for 2.0 BTC
//     let sell_order = Order {
//         id: 1,
//         price: 100.0,
//         quantity: 2.0,
//         remaining_quantity: 2.0,
//         side: Side::Sell,
//         order_type: OrderType::Limit,
//         status: OrderStatus::New,
//         timestamp: 1,
//     };

//     println!("Adding sell order...");
//     let trades = orderbook.add_order(sell_order);
//     println!("Trades generated: {:?}", trades);

//     // 2. Place a matching Buy Order (Bid) at $100 for 1.5 BTC
//     let buy_order = Order {
//         id: 2,
//         price: 100.0,
//         quantity: 1.5,
//         remaining_quantity: 1.5,
//         side: Side::Buy,
//         order_type: OrderType::Limit,
//         status: OrderStatus::New,
//         timestamp: 2,
//     };

//     println!("Adding matching buy order...");
//     let trades = orderbook.add_order(buy_order);
//     println!("Trades generated: {:?}", trades);
// }

// ////////////////////////////////////////////


