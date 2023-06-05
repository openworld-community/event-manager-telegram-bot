use url::{Url, ParseError};


pub struct Transaction {
  sender_address: String,
  receiving_address: String,
  amount: f64,
  currency: String,
  status: String, // Temp
}

// TBD
