

use std::time::{SystemTime,UNIX_EPOCH};
use std::env;
use jsonwebtoken::{encode, EncodingKey, Header};
// use models::Claims;
use crate::models::auth::Claim;




pub fn create_jwt(id:u32)->Result<String, jsonwebtoken::errors::Error>{

    //expiry token time from now
    let now= SystemTime::now().duration_since(UNIX_EPOCH).expect("Time moved backward");
// let expiry:  = env::var("TOKEN_EXPIRY_TIME")
//     we take variable from the .env file using let xyz:Result<String, VarError> = env::var('variable name');
//taking token expiry time from .env
let expiry:u64= env::var("TOKEN_EXPIRY_TIME").expect("token expiry time must be .env file").parse().expect("xyz");
let expiration=now.as_secs()+expiry;


let current_timestamp = now.as_secs();
let jwt_secret:String= env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");

//user id
let claims = Claim {
    sub: id,
    exp: expiration,
    iat:current_timestamp,
};

encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_bytes()))

}
