use serde::{Deserialize, Serialize};

use actix_web::{ dev::{ServiceRequest, ServiceResponse}, body::{MessageBody,
    EitherBody},error::ErrorUnauthorized, middleware::Next, Error, App, HttpResponse,HttpRequest, HttpServer,HttpMessage, Responder,};


pub async fn protect<B>(req:ServiceRequest,next:Next<B>)
->Result<ServiceResponse<EitherBody<B>>,Error>
where B:MessageBody+'static,
{
    let auth= match req.headers().get("Authorization"){
        Some(auth)=>auth,
        None=>{
            // return Err(ErrorUnauthorized("Missing Authorization Header"));
             return Ok(req.into_response(
    HttpResponse::Unauthorized().json(ClientResponse {
        message: String::from("Missing Authorization Header"),
    })
).map_into_right_body()); 
        }

    };
    //heare is in the header value now we turn it in to str from Header value type
let auth= match auth.to_str(){
    Ok(auth)=>auth,
    Err(_)=>{
        
        return Ok(req.into_response(
    HttpResponse::Unauthorized().json(ClientResponse {
        message: String::from("Missing Authorization Header"),
    })
).map_into_right_body()); 
    }
};

//Cheking second condition authorizatio not startes with Bearer
    if !auth.starts_with("Bearer "){
        return Ok(req.into_response(
    HttpResponse::Unauthorized().json(ClientResponse {
        message: String::from("Missing Authorization Header"),
    })
).map_into_right_body()); 
 }

 let token= &auth["Bearer ".len()..];

 //if we do not get token
 if token.is_empty(){
    return Ok(req.into_response(
    HttpResponse::Unauthorized().json(ClientResponse {
        message: String::from("Do not have the JWT login token"),
    })
).map_into_right_body()); 

 }

//NOW CHECK OUT TOKEN IS VALID OR NOT
let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

 let decoded= match decode::<Claim>(token, &DecodingKey::from_secret(jwt_secret.as_bytes()),
 &Validation::default(),){
    Ok(data)=>data,
    Err(err)=>{
        println!("JWT ERROR: {:?}", err);
        return Ok(req.into_response(
            HttpResponse::Unauthorized().json(ClientResponse {
                message:String::from("Invalid token"),
            })
        ).map_into_right_body());
    }
 };

    // println!("{:?}", decoded.claims);
    // "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTc4NzI4NTYwNX0.NsrMDDF_FzNw6jV6CsoAxDxQ13KY3buYu3c5zWACQsQ"
//out of decode.claims= { sub: 1, exp: 1787306595, iat: 1787220195 }

let app_state = req
        .app_data::<web::Data<AppState>>()
        .expect("AppState not configured");

let user_id=decoded.claims.sub;
let token_issue_time=decoded.claims.iat;

//we smartly used the conditioning, 
//01 we have user valid password changed so token is invalid not allowed to access using token
//02 we have tokenbut user deleted his account after token creation/login not allowed to acces
//in other conditions we allowed to login, so i all condition we need to check user 
let user_check={
    let users=app_state.users.lock().unwrap();
    match users.iter().find(|u|u.id==user_id){
        Some(user)=>{
            //user exist
            match user.pass_change_at{
                Some(timestamp) if timestamp > token_issue_time=>{
            
                    //user exist but passwors was changed
                    2
                }

                _=>{
                    //User Exist and JWT is Okay
                    1
                }

            }
        },

        None=>{
            //user Does not exist 
            0

        }
    }
};

if user_check==0 {
    return Ok(
        req.into_response(
            HttpResponse::Unauthorized().json(ClientResponse {
                message: String::from("User not exists, please signup again"),
            })
        )
        .map_into_right_body()
    );
}

if user_check==2 {
    return Ok(
        req.into_response(
            HttpResponse::Unauthorized().json(ClientResponse {
                message: String::from("Password changed login with new password"),
            })
        )
        .map_into_right_body()
    );
}

//for temporarly 
// if user_check==1 {
//     return Ok(
//         req.into_response(
//             HttpResponse::Ok().json(ClientResponse {
//                 message: String::from("User exist and token also exist"),
//             })
//         )
//         .map_into_right_body()
//     );
// }

println!("Middleware authenticated user: {}", user_id);

req.extensions_mut().insert(user_id);

println!(
    "Extension before next: {:?}",
    req.extensions().get::<u32>()
);

next.call(req)
    .await
    .map(|res| res.map_into_left_body())

// req.extensions_mut().insert(user_id);

//      next.call(req).await.map(|res|res.map_into_left_body())
}



///////Signup////////////////
/// 
/// 
