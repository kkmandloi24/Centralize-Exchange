// mod models;

use crate::models::{SignupRequest,SignupResponse,ClientResponse, LoginResponse};
use crate::models::users::{User};
use actix_web::{web, HttpResponse, Responder};
use crate::utils::jwt::create_jwt;
use crate::db::AppState;

use actix_web::error::{ErrorInternalServerError,};
use bcrypt::{hash,verify};

//1). SIGNUP
pub async fn sign_up(body: web::Json<SignupRequest>, app_state:web::Data<AppState>) -> impl Responder {
 
    //locking the app_state variable
    let mut users=app_state.users.lock().unwrap();

    //lock the user id creation/till this thread complete
    let mut user_index=app_state.user_index.lock().unwrap();

let user_found= users.iter().find(|u| u.username == body.username);
//     println!("User not exist create the user")


    if user_found.is_some() {
       return HttpResponse::Conflict().json(SignupResponse {
           message: String::from("User already exists"),
           token: None,
       })

};

  
//existing user
// let user= match users.iter().find(|u| u.username == body.username){
//   Some(user)=>{
//     return Err(ErrorConflict("User {user} already exists"));
//   }
//   None=>{
//     println!("User not exist create the user")


//   }
// };

  

//if user not exist/user is first time
*user_index += 1;
let new_user_id = *user_index;
    //  let hashed_password=  hash(&body.password,12).map_err(|e| format!("Password hashing failed: {e}"))?;
     
     //creating hashed password//gives result need to handle ok and Err
     let hashed_password= match hash(&body.password, 12){
        Ok(hashed_password)=>hashed_password,
        Err(err) => {
        println!("There was an error creating the hashed password: {}", err);

    return HttpResponse::InternalServerError().json(ClientResponse {
    message: String::from("Something went wrong"),
})
    },
  };
         
     
//saving user in the DB
    users.push(User{
        id:new_user_id,
        username:body.username.clone(),
        password:hashed_password,
        pass_change_at: None,
       
    });
    //  pass_change_at:Option<u64>
    drop(users);
    drop(user_index);

    
//Create Token
  let token= match create_jwt(new_user_id){
    Ok(token)=>token,
    Err(err)=>{
        println!("JWT creation failed {}", err);
        return HttpResponse::InternalServerError().json(ClientResponse {
    message: String::from("Could not create a token"),
})

}
// Return success response with the created token
    // HttpResponse::Ok().json(SignupResponse {
    //     message: String::from("User registered successfully"),
    //     token: Some(token), // Wrap your token in Some()
    // })
    

  };
 
   return HttpResponse::Created().json(SignupResponse {
        message: String::from("Successfully Signup"),
        token: Some(token),
    })
      
}


//2) LOGIN
pub async fn login(body: web::Json<SignupRequest>, app_state:web::Data<AppState>) -> impl Responder {

    let  users=app_state.users.lock().unwrap();

  let user= match users.iter().find(|u| u.username==body.username){
    Some(user)=>user,
    None=>{
        return HttpResponse::Unauthorized().json(ClientResponse{
            message:String::from("InValid username and password"),
            
        })
    }
  };

 

  //matching the password user inputs password onlogin field and one which is saved in DB
  //Db password is hashed , so we have to hashed he input password and compare them



  let correct_password= match verify(&body.password, &user.password){
    Ok(correct_password)=>correct_password,
    Err(_)=>{
        return HttpResponse::Unauthorized().json(ClientResponse{
            message:String::from("InValid password"),
           
        })

    }

  };

    if !correct_password{
         return HttpResponse::Unauthorized().json(ClientResponse {
        message: String::from("Invalid username and password"),

    });

    }

    //If User is ahs right password and username then we create the token

  let user_id= user.id;

  drop(users);

  let token= match create_jwt(user_id){
    Ok(token)=>token,
    Err(err)=>{
        println!("JWT creation failed: {}", err);
        return HttpResponse::InternalServerError().json(ClientResponse{
            message:String::from("could not create token"),

        })
    }
  };


        HttpResponse::Ok().json(LoginResponse {
        message: String::from(" Login Successfully"),
        token: Some(token),
    })

    }


