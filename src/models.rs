#[derive(Debug, sqlx::FromRow)]
pub struct SeenPost {
    pub id: i32,
    pub link: String,
}
