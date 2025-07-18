use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct UserStorage {
    pub id: i64,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
}
