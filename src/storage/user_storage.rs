#[derive(Deserialize, Serialize, Debug, Clone, FromRow, PartialEq)]
pub struct UserInternal {
    #[serde(default)]
    pub id: i64,
    pub profile_url: String,
    #[serde(default)]
    pub screen_name: String,
    #[serde(default)]
    pub profile_image_url: String,
    #[serde(default)]
    pub avatar_large: String,
    #[serde(default)]
    pub avatar_hd: String,
    #[sqlx(default)]
    #[serde(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    #[serde(default, deserialize_with = "parse_v_plus")]
    pub v_plus: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub verified: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub domain: String,
    #[sqlx(default)]
    #[serde(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    #[serde(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub following: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub mbtype: i64,
    pub icon_list: Option<Value>,
    #[sqlx(default)]
    #[serde(default)]
    pub backedup: bool,
}

impl UserInternal {
    pub async fn create_table<E>(mut db: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users ( \
             id INTEGER PRIMARY KEY, \
             profile_url TEXT, \
             screen_name TEXT, \
             profile_image_url TEXT, \
             avatar_large TEXT, \
             avatar_hd TEXT, \
             planet_video INTEGER, \
             v_plus INTEGER, \
             pc_new INTEGER, \
             verified INTEGER, \
             verified_type INTEGER, \
             domain TEXT, \
             weihao TEXT, \
             verified_type_ext INTEGER, \
             follow_me INTEGER, \
             following INTEGER, \
             mbrank INTEGER, \
             mbtype INTEGER, \
             icon_list TEXT, \
             backedup INTEGER \
             )",
        )
        .execute(&mut *db)
        .await?;
        Ok(())
    }
}
