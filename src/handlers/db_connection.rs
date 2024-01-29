use std::fs;

use mongodb::{Client, Database};

//connect to database in mongodb..............................................................................
pub async fn blockchain_db() -> Result<Database, String> {
    let result = Client::with_uri_str("mongodb://localhost:27017").await;
    let check_mongo = fs::metadata("c:\\Program Files\\MongoDB\\Server\\7.0\\bin");

    match result {
        Ok(client) => {
            match check_mongo {
                Err(_) => {
                    return Err("Mongodb is not install yet! please check your mongodb and get the latest version or remove it completely and run centichain again".to_string())
                }
                Ok(_) => {
                    let database = client.database("Blockchain");
                    return Ok(database);
                }
            }
        }
        Err(_) => return Err("Mongodb is not install yet! please check your mongodb and get the latest version or remove it completely and run centichain again".to_string())
    }
}
