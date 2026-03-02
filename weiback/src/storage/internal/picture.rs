use std::path::PathBuf;

use sqlx::{Executor, Sqlite};
use url::Url;

use crate::error::{Error, Result};
use crate::models::{PictureDefinition, PictureMeta};
use crate::storage::PictureInfo;
use crate::utils::{pic_url_to_db_key, pic_url_to_id};

#[derive(sqlx::FromRow, Debug)]
struct PictureDbRecord {
    url: String,
    path: Option<String>,
    post_id: Option<i64>,
    user_id: Option<i64>,
    definition: Option<String>,
}

impl TryFrom<PictureDbRecord> for PictureInfo {
    type Error = Error;

    fn try_from(record: PictureDbRecord) -> std::result::Result<Self, Self::Error> {
        let path = record.path.ok_or_else(|| {
            Error::DbError(format!("Picture path is NULL for URL {}", record.url))
        })?;
        let url_obj = Url::parse(&record.url)?;
        let meta = match record {
            PictureDbRecord {
                post_id: Some(post_id),
                definition: Some(definition),
                user_id: None,
                ..
            } => PictureMeta::Attached {
                url: url_obj,
                post_id,
                definition: PictureDefinition::from(definition.as_str()),
            },
            PictureDbRecord {
                post_id: Some(post_id),
                definition: None,
                user_id: None,
                ..
            } => PictureMeta::Cover {
                url: url_obj,
                post_id,
            },
            PictureDbRecord {
                user_id: Some(user_id),
                post_id: None,
                definition: None,
                ..
            } => PictureMeta::Avatar {
                url: url_obj,
                user_id,
            },
            PictureDbRecord {
                post_id: None,
                user_id: None,
                definition: None,
                ..
            } => PictureMeta::Other { url: url_obj },
            _ => unreachable!(),
        };
        Ok(PictureInfo {
            meta,
            path: PathBuf::from(path),
        })
    }
}

pub async fn save_picture_meta<'e, E>(
    executor: E,
    picture_meta: &PictureMeta,
    relative_path_str: Option<&str>,
) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (url, post_id, user_id, definition) = match picture_meta {
        PictureMeta::Attached {
            url,
            definition,
            post_id,
        } => (url, Some(*post_id), None, Some(definition)),
        PictureMeta::Cover { url, post_id } => (url, Some(*post_id), None, None),
        PictureMeta::Avatar { url, user_id } => (url, None, Some(*user_id), None),
        PictureMeta::Other { url } => (url, None, None, None),
    };
    let url_str = pic_url_to_db_key(url).to_string();
    sqlx::query(
        r#"INSERT OR IGNORE INTO picture (
    id,
    path,
    post_id,
    url,
    user_id,
    definition
)
VALUES
    (?, ?, ?, ?, ?, ?);"#,
    )
    .bind(pic_url_to_id(picture_meta.url()).unwrap_or_default())
    .bind(relative_path_str)
    .bind(post_id)
    .bind(url_str)
    .bind(user_id)
    .bind(definition.map(<&str>::from))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_picture_path<'e, E>(executor: E, url: &Url) -> Result<Option<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let raw_res =
        sqlx::query_scalar::<Sqlite, String>(r#"SELECT path FROM picture WHERE url = ?;"#)
            .bind(url.as_str())
            .fetch_optional(executor)
            .await?;
    Ok(raw_res.map(PathBuf::from))
}

pub async fn get_users_with_duplicate_avatars<'e, E>(executor: E) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let ids = sqlx::query_scalar::<Sqlite, i64>(
        "SELECT user_id FROM picture WHERE user_id IS NOT NULL GROUP BY user_id HAVING COUNT(user_id) > 1",
    )
    .fetch_all(executor)
    .await?;
    Ok(ids)
}

pub async fn get_pictures_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records: Vec<PictureDbRecord> =
        sqlx::query_as("SELECT * FROM picture WHERE post_id = ? AND path IS NOT NULL")
            .bind(post_id)
            .fetch_all(executor)
            .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_avatars_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records: Vec<PictureDbRecord> = sqlx::query_as(
        "SELECT * FROM picture WHERE user_id = ? AND post_id IS NULL AND path IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_avatar_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Option<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let record: Option<PictureDbRecord> = sqlx::query_as(
        "SELECT * FROM picture WHERE user_id = ? AND post_id IS NULL AND path IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;
    record.map(PictureInfo::try_from).transpose()
}

pub async fn get_pictures_by_ids<'e, E>(executor: E, ids: &[String]) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT * FROM picture WHERE id IN ({}) AND path IS NOT NULL",
        placeholders
    );

    let mut query = sqlx::query_as::<_, PictureDbRecord>(&sql);
    for id in ids {
        query = query.bind(id);
    }
    let records = query.fetch_all(executor).await?;

    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_pictures_by_id<'e, E>(executor: E, id: &str) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records = sqlx::query_as::<_, PictureDbRecord>(
        "SELECT * FROM picture WHERE id = ? AND path IS NOT NULL",
    )
    .bind(id)
    .fetch_all(executor)
    .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn delete_pictures_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM picture WHERE post_id = ?")
        .bind(post_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn get_duplicate_pic_ids<'e, E>(executor: E) -> Result<Vec<String>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let ids = sqlx::query_scalar::<Sqlite, String>(
        "SELECT id FROM picture WHERE id != '' GROUP BY id HAVING COUNT(id) > 1",
    )
    .fetch_all(executor)
    .await?;
    Ok(ids)
}

pub async fn delete_picture_by_url<'e, E>(executor: E, url: &Url) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM picture WHERE url = ?")
        .bind(url.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

#[cfg(test)]
mod local_tests {
    use sqlx::sqlite::SqlitePool;

    use super::*;
    use crate::storage::database::create_db_pool_with_url;

    async fn setup_db() -> SqlitePool {
        create_db_pool_with_url("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_save_and_get_picture() {
        let db = setup_db().await;
        let url = Url::parse("http://example.com/pic.jpg").unwrap();
        let meta = PictureMeta::Other { url: url.clone() };
        let path = "some/path/pic.jpg";

        save_picture_meta(&db, &meta, Some(path)).await.unwrap();

        let retrieved_path = get_picture_path(&db, &url).await.unwrap();
        assert_eq!(retrieved_path, Some(PathBuf::from(path)));
    }

    #[tokio::test]
    async fn test_picture_record_conversion() {
        let url_str = "http://example.com/image.jpg";
        let path = "image.jpg";

        // Test Attached
        let record = PictureDbRecord {
            url: url_str.to_string(),
            path: Some(path.to_string()),
            post_id: Some(123),
            user_id: None,
            definition: Some("large".to_string()),
        };
        let info = PictureInfo::try_from(record).unwrap();
        assert_eq!(info.path, PathBuf::from(path));
        match info.meta {
            PictureMeta::Attached {
                url,
                post_id,
                definition,
            } => {
                assert_eq!(url.as_str(), url_str);
                assert_eq!(post_id, 123);
                assert_eq!(definition, PictureDefinition::Large);
            }
            _ => panic!("Wrong PictureMeta type"),
        }

        // Test Cover
        let record = PictureDbRecord {
            url: url_str.to_string(),
            path: Some(path.to_string()),
            post_id: Some(123),
            user_id: None,
            definition: None,
        };
        let info = PictureInfo::try_from(record).unwrap();
        assert_eq!(info.path, PathBuf::from(path));
        match info.meta {
            PictureMeta::Cover { url, post_id } => {
                assert_eq!(url.as_str(), url_str);
                assert_eq!(post_id, 123);
            }
            _ => panic!("Wrong PictureMeta type"),
        }

        // Test Avatar
        let record = PictureDbRecord {
            url: url_str.to_string(),
            path: Some(path.to_string()),
            post_id: None,
            user_id: Some(456),
            definition: None,
        };
        let info = PictureInfo::try_from(record).unwrap();
        assert_eq!(info.path, PathBuf::from(path));
        match info.meta {
            PictureMeta::Avatar { url, user_id } => {
                assert_eq!(url.as_str(), url_str);
                assert_eq!(user_id, 456);
            }
            _ => panic!("Wrong PictureMeta type"),
        }

        // Test Other
        let record = PictureDbRecord {
            url: url_str.to_string(),
            path: Some(path.to_string()),
            post_id: None,
            user_id: None,
            definition: None,
        };
        let info = PictureInfo::try_from(record).unwrap();
        assert_eq!(info.path, PathBuf::from(path));
        match info.meta {
            PictureMeta::Other { url } => {
                assert_eq!(url.as_str(), url_str);
            }
            _ => panic!("Wrong PictureMeta type"),
        }

        // Test missing path
        let record = PictureDbRecord {
            url: url_str.to_string(),
            path: None,
            post_id: None,
            user_id: None,
            definition: None,
        };
        assert!(PictureInfo::try_from(record).is_err());
    }

    #[tokio::test]
    async fn test_get_pictures_by_post_id() {
        let db = setup_db().await;
        let post_id = 123;
        let url1 = Url::parse("http://example.com/pic1.jpg").unwrap();
        let url2 = Url::parse("http://example.com/pic2.jpg").unwrap();
        let meta1 = PictureMeta::Attached {
            url: url1,
            post_id,
            definition: PictureDefinition::Large,
        };
        let meta2 = PictureMeta::Cover { url: url2, post_id };

        save_picture_meta(&db, &meta1, Some("p1")).await.unwrap();
        save_picture_meta(&db, &meta2, Some("p2")).await.unwrap();

        let pictures = get_pictures_by_post_id(&db, post_id).await.unwrap();
        assert_eq!(pictures.len(), 2);
    }

    #[tokio::test]
    async fn test_avatar_functions() {
        let db = setup_db().await;
        let user_id1 = 1;
        let user_id2 = 2;

        // Save avatars
        let url1 = Url::parse("http://example.com/avatar1.jpg").unwrap();
        let meta1 = PictureMeta::Avatar {
            url: url1.clone(),
            user_id: user_id1,
        };
        save_picture_meta(&db, &meta1, Some("avatar1.jpg"))
            .await
            .unwrap();

        // one user with two avatars
        let url2 = Url::parse("http://example.com/avatar2.jpg").unwrap();
        let meta2 = PictureMeta::Avatar {
            url: url2.clone(),
            user_id: user_id2,
        };
        save_picture_meta(&db, &meta2, Some("avatar2.jpg"))
            .await
            .unwrap();
        let url3 = Url::parse("http://example.com/avatar3.jpg").unwrap();
        let meta3 = PictureMeta::Avatar {
            url: url3.clone(),
            user_id: user_id2,
        };
        save_picture_meta(&db, &meta3, Some("avatar3.jpg"))
            .await
            .unwrap();

        // get_avatars_by_user_id
        let avatars1 = get_avatars_by_user_id(&db, user_id1).await.unwrap();
        assert_eq!(avatars1.len(), 1);
        let avatars2 = get_avatars_by_user_id(&db, user_id2).await.unwrap();
        assert_eq!(avatars2.len(), 2);

        // get_avatar_by_user_id (should return the first one)
        let avatar1 = get_avatar_by_user_id(&db, user_id1).await.unwrap();
        assert!(avatar1.is_some());

        // get_users_with_duplicate_avatars
        let duplicate_users = get_users_with_duplicate_avatars(&db).await.unwrap();
        assert_eq!(duplicate_users, vec![user_id2]);
    }

    #[tokio::test]
    async fn test_get_pictures_by_ids() {
        let db = setup_db().await;
        let url1 = Url::parse("http://example.com/pic1.jpg").unwrap();
        let id1 = pic_url_to_id(&url1).unwrap();
        let meta1 = PictureMeta::Other { url: url1 };
        save_picture_meta(&db, &meta1, Some("p1")).await.unwrap();

        let url2 = Url::parse("http://example.com/pic2.jpg").unwrap();
        let id2 = pic_url_to_id(&url2).unwrap();
        let meta2 = PictureMeta::Other { url: url2 };
        save_picture_meta(&db, &meta2, Some("p2")).await.unwrap();

        let ids = vec![id1.clone(), id2.clone()];
        let pictures = get_pictures_by_ids(&db, &ids).await.unwrap();
        assert_eq!(pictures.len(), 2);

        let pictures = get_pictures_by_id(&db, &id1).await.unwrap();
        assert_eq!(pictures.len(), 1);
    }

    #[tokio::test]
    async fn test_get_duplicate_pic_ids() {
        let db = setup_db().await;
        let url1 = Url::parse("http://example.com/duplicate.jpg").unwrap();
        let id = pic_url_to_id(&url1).unwrap();

        let meta1 = PictureMeta::Other { url: url1 };
        save_picture_meta(&db, &meta1, Some("p1")).await.unwrap();

        let url2 = Url::parse("http://example2.com/duplicate.png").unwrap();
        let meta2 = PictureMeta::Other { url: url2 };
        save_picture_meta(&db, &meta2, Some("p2")).await.unwrap();

        let duplicates = get_duplicate_pic_ids(&db).await.unwrap();
        assert_eq!(duplicates, vec![id]);
    }

    #[tokio::test]
    async fn test_delete_functions() {
        let db = setup_db().await;
        let post_id = 999;
        let url1 = Url::parse("http://example.com/todelete1.jpg").unwrap();
        let meta1 = PictureMeta::Attached {
            url: url1.clone(),
            post_id,
            definition: PictureDefinition::Large,
        };
        save_picture_meta(&db, &meta1, Some("p1")).await.unwrap();

        let url2 = Url::parse("http://example.com/todelete2.jpg").unwrap();
        let meta2 = PictureMeta::Other { url: url2.clone() };
        save_picture_meta(&db, &meta2, Some("p2")).await.unwrap();

        // Test delete by post_id
        delete_pictures_by_post_id(&db, post_id).await.unwrap();
        let pictures = get_pictures_by_post_id(&db, post_id).await.unwrap();
        assert!(pictures.is_empty());

        // Test delete by url
        let path_before_delete = get_picture_path(&db, &url2).await.unwrap();
        assert!(path_before_delete.is_some());
        delete_picture_by_url(&db, &url2).await.unwrap();
        let path_after_delete = get_picture_path(&db, &url2).await.unwrap();
        assert!(path_after_delete.is_none());
    }
}
