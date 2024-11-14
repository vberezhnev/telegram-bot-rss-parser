use sqlx::{Pool, Postgres};
use std::env;

pub async fn establish_connection() -> Result<Pool<Postgres>, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let pool = Pool::<Postgres>::connect(&database_url).await?;
    Ok(pool)
}
