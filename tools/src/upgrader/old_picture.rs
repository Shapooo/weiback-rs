use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OldPictureRecord {
    #[allow(unused)]
    pub id: String,
    pub url: String,
    pub uid: Option<i64>,
    pub post_id: Option<i64>,
    pub blob: Vec<u8>,
}

pub async fn get_old_pictures_paged(
    db: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<OldPictureRecord>> {
    let records = sqlx::query_as(
        r#"SELECT
    p.id,
    pb.url,
    p.uid,
    p.post_id,
    pb.blob
FROM
    picture AS p
    LEFT JOIN picture_blob AS pb ON p.id = pb.id LIMIT ?
OFFSET
    ?;"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;
    Ok(records)
}
