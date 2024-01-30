use mongodb::{Client, Database};

//connect to database in mongodb..............................................................................
pub async fn blockchain_db() -> Result<Database, String> {
    let result = Client::with_uri_str("mongodb://localhost:27017").await;

    match result {
        Ok(client) => {
            let database = client.database("Blockchain");
            return Ok(database);
        }
        Err(_) => return Err("mongodb connection error!".to_string()),
    }
}
