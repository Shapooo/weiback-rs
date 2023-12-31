use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlUser {
    pub id: i64,
    pub profile_url: String,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    pub v_plus: i64,
    #[sqlx(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
    #[sqlx(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    pub mbtype: i64,
    #[sqlx(default)]
    pub icon_list: String,
    #[sqlx(default)]
    pub backedup: bool,
}
