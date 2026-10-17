use serde::{Deserialize, Serialize};

/// Request payload for user registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Response payload returned after a successful registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupResponse {
    pub message: String,
    // pub user_id: i32, // Change to String / Uuid if using UUIDs
    pub token: Option<String>,
}

/// Response payload representing client information or status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientResponse {
    // pub id: i32,
    // pub name: String,
    // pub status: String,
    pub message:String,
}

/// Response payload returned after a successful login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: Option<String>,
    // pub token_type: String, // e.g., "Bearer"
    // pub expires_in: u64, 
    pub message:String,   // Expiration time in seconds
}

/// JWT Claims structure used for token encoding and decoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub sub: u32,    // Subject (usually user ID or username)
    pub exp: u64,     // Expiration timestamp (Unix time)
    pub iat: u64, // Issued at timestamp
    // Add any extra custom claims here (e.g., role, email, etc.)
    // pub role: Option<String>,
}