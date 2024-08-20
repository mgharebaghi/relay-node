use mongodb::{Client, Database};

pub struct Mongodb;

impl Mongodb {
    pub async fn connect<'a>() -> Result<Database, &'a str> {
        let uri = "mongodb://localhost:27017";
        let connection = Client::with_uri_str(uri).await;

        match connection {
            Ok(client) => {
                let db = client.database("Centichain");
                Ok(db)
            }
            Err(_) => Err("Database Connection Problem-(db-12)"),
        }
    }
}
