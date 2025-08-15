use chrono::DateTime;
use log::{debug, info};
use serde_json::{Value, from_value, to_value};
use sqlx::{FromRow, Sqlite, SqlitePool};

use crate::error::{Error, Result};
use crate::models::Post;

#[derive(Debug, Clone, PartialEq, FromRow, serde::Serialize, serde::Deserialize)]
pub struct PostInternal {
    pub id: i64,
    pub mblogid: String,
    pub source: Option<String>,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub pic_ids: Option<Value>,
    pub pic_num: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub text: String,
    pub attitudes_status: i64,
    pub favorited: bool,
    pub pic_infos: Option<Value>,
    pub reposts_count: Option<i64>,
    pub comments_count: Option<i64>,
    pub attitudes_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub edit_count: Option<i64>,
    #[sqlx(rename = "isLongText")]
    pub is_long_text: bool,
    pub geo: Option<Value>,
    pub page_info: Option<Value>,
    pub unfavorited: bool,
    pub created_at: String,
    pub retweeted_id: Option<i64>,
    pub uid: Option<i64>,
}

impl TryFrom<Post> for PostInternal {
    type Error = Error;
    fn try_from(post: Post) -> Result<Self> {
        Ok(Self {
            id: post.id,
            mblogid: post.mblogid,
            source: post.source,
            region_name: post.region_name,
            deleted: post.deleted,
            pic_ids: post.pic_ids.map(|v| to_value(&v)).transpose()?,
            pic_num: post.pic_num,
            url_struct: post.url_struct,
            topic_struct: post.topic_struct,
            tag_struct: post.tag_struct,
            number_display_strategy: post.number_display_strategy,
            mix_media_info: post.mix_media_info,
            text: post.text,
            attitudes_status: post.attitudes_status,
            favorited: post.favorited,
            pic_infos: post.pic_infos.map(|h| to_value(&h)).transpose()?,
            reposts_count: post.reposts_count,
            comments_count: post.comments_count,
            attitudes_count: post.attitudes_count,
            repost_type: post.repost_type,
            edit_count: post.edit_count,
            is_long_text: post.is_long_text,
            geo: post.geo,
            page_info: post.page_info,
            unfavorited: post.unfavorited,
            created_at: post.created_at.to_rfc3339(),
            retweeted_id: post.retweeted_status.map(|r| r.id),
            uid: post.user.map(|u| u.id),
        })
    }
}

impl TryInto<Post> for PostInternal {
    type Error = Error;
    fn try_into(self) -> Result<Post> {
        Ok(Post {
            id: self.id,
            mblogid: self.mblogid,
            source: self.source,
            region_name: self.region_name,
            deleted: self.deleted,
            pic_ids: self.pic_ids.map(from_value).transpose()?,
            pic_num: self.pic_num,
            url_struct: self.url_struct,
            topic_struct: self.topic_struct,
            tag_struct: self.tag_struct,
            number_display_strategy: self.number_display_strategy,
            mix_media_info: self.mix_media_info,
            text: self.text,
            attitudes_status: self.attitudes_status,
            favorited: self.favorited,
            pic_infos: self.pic_infos.map(from_value).transpose()?,
            reposts_count: self.reposts_count,
            comments_count: self.comments_count,
            attitudes_count: self.attitudes_count,
            repost_type: self.repost_type,
            edit_count: self.edit_count,
            is_long_text: self.is_long_text,
            geo: self.geo,
            page_info: self.page_info,
            unfavorited: self.unfavorited,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?,
            retweeted_status: None,
            user: None,
        })
    }
}

pub async fn create_post_table(db: &SqlitePool) -> Result<()> {
    info!("Creating post table if not exists...");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts ( \
         id INTEGER PRIMARY KEY, \
         mblogid TEXT, \
         source TEXT, \
         region_name TEXT, \
         deleted INTEGER, \
         pic_ids TEXT, \
         pic_num INTEGER, \
         url_struct TEXT, \
         topic_struct TEXT, \
         tag_struct TEXT, \
         number_display_strategy TEXT, \
         mix_media_info TEXT, \
         text TEXT, \
         attitudes_status INTEGER, \
         favorited INTEGER, \
         pic_infos TEXT, \
         reposts_count INTEGER, \
         comments_count INTEGER, \
         attitudes_count INTEGER, \
         repost_type INTEGER, \
         edit_count INTEGER, \
         isLongText INTEGER, \
         geo TEXT, \
         page_info TEXT, \
         unfavorited INTEGER, \
         created_at TEXT, \
         retweeted_id INTEGER, \
         uid INTEGER \
         )",
    )
    .execute(db)
    .await?;
    info!("Post table created successfully.");
    Ok(())
}

pub async fn get_post(db: &SqlitePool, id: i64) -> Result<Option<PostInternal>> {
    Ok(
        sqlx::query_as::<Sqlite, PostInternal>("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

pub async fn get_favorites(
    db: &SqlitePool,
    limit: u32,
    offset: u32,
    reverse: bool,
) -> Result<Vec<PostInternal>> {
    debug!("query favorites offset {offset}, limit {limit}, rev {reverse}");
    let sql_expr = if reverse {
        "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
    } else {
        "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
    };
    let posts = sqlx::query_as::<Sqlite, PostInternal>(sql_expr)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
    Ok(posts)
}

pub async fn get_posts(
    db: &SqlitePool,
    limit: u32,
    offset: u32,
    reverse: bool,
) -> Result<Vec<PostInternal>> {
    debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
    let sql_expr = if reverse {
        "SELECT * FROM posts ORDER BY id LIMIT ? OFFSET ?"
    } else {
        "SELECT * FROM posts ORDER BY id DESC LIMIT ? OFFSET ?"
    };
    let posts = sqlx::query_as::<Sqlite, PostInternal>(sql_expr)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
    Ok(posts)
}

pub async fn get_ones_posts(
    db: &SqlitePool,
    uid: i64,
    limit: u32,
    offset: u32,
    reverse: bool,
) -> Result<Vec<PostInternal>> {
    debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
    let sql_expr = if reverse {
        "SELECT * FROM posts WHERE uid = ? ORDER BY id LIMIT ? OFFSET ?"
    } else {
        "SELECT * FROM posts WHERE uid = ? ORDER BY id DESC LIMIT ? OFFSET ?"
    };
    let posts = sqlx::query_as::<Sqlite, PostInternal>(sql_expr)
        .bind(uid)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
    Ok(posts)
}

pub async fn save_post(db: &SqlitePool, post: &PostInternal, overwrite: bool) -> Result<()> {
    sqlx::query(
        format!(
            "INSERT OR {} INTO posts (\
                 id,\
                 mblogid,\
                 source,\
                 region_name,\
                 deleted,\
                 pic_ids,\
                 pic_num,\
                 url_struct,\
                 topic_struct,\
                 tag_struct,\
                 number_display_strategy,\
                 mix_media_info,\
                 text,\
                 attitudes_status,\
                 favorited,\
                 pic_infos,\
                 reposts_count,\
                 comments_count,\
                 attitudes_count,\
                 repost_type,\
                 edit_count,\
                 isLongText,\
                 geo,\
                 page_info,\
                 unfavorited,\
                 created_at,\
                 retweeted_id,\
                 uid)\
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?,\
                 ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,\
                 ?, ?, ?, ?, ?, ?, ?, ?)",
            if overwrite { "REPLACE" } else { "IGNORE" }
        )
        .as_str(),
    )
    .bind(post.id)
    .bind(&post.mblogid)
    .bind(&post.source)
    .bind(&post.region_name)
    .bind(post.deleted)
    .bind(&post.pic_ids)
    .bind(post.pic_num)
    .bind(&post.url_struct)
    .bind(&post.topic_struct)
    .bind(&post.tag_struct)
    .bind(&post.number_display_strategy)
    .bind(&post.mix_media_info)
    .bind(&post.text)
    .bind(post.attitudes_status)
    .bind(post.favorited)
    .bind(&post.pic_infos)
    .bind(post.reposts_count)
    .bind(post.comments_count)
    .bind(post.attitudes_count)
    .bind(post.repost_type)
    .bind(post.edit_count)
    .bind(post.is_long_text)
    .bind(&post.geo)
    .bind(&post.page_info)
    .bind(post.unfavorited)
    .bind(&post.created_at)
    .bind(post.retweeted_id)
    .bind(post.uid)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn mark_post_unfavorited(db: &SqlitePool, id: i64) -> Result<()> {
    debug!("unfav post {id} in db");
    sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn mark_post_favorited(db: &SqlitePool, id: i64) -> Result<()> {
    debug!("mark favorited post {id} in db");
    sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_favorited_sum(db: &SqlitePool) -> Result<u32> {
    Ok(
        sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
            .fetch_one(db)
            .await?
            .0,
    )
}

pub async fn get_posts_id_to_unfavorite(db: &SqlitePool) -> Result<Vec<i64>> {
    debug!("query all posts to unfavorite");
    Ok(sqlx::query_as::<Sqlite, (i64,)>(
        "SELECT id FROM posts WHERE unfavorited == false and favorited;",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|t| t.0)
    .collect())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use sqlx::SqlitePool;
    use weibosdk_rs::{
        Post,
        favorites::FavoritesAPI,
        mock::{MockAPI, MockClient},
        profile_statuses::ProfileStatusesAPI,
    };

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_post_table(&pool).await.unwrap();
        pool
    }

    async fn create_test_posts() -> Vec<Post> {
        let mut posts = Vec::new();
        let client = MockClient::new();
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        client
            .set_favorites_response_from_file(
                manifest_dir.join("tests/data/favorites.json").as_path(),
            )
            .unwrap();
        client
            .set_profile_statuses_response_from_file(
                manifest_dir
                    .join("tests/data/profile_statuses.json")
                    .as_path(),
            )
            .unwrap();
        let api = MockAPI::from_session(client, Default::default());
        posts.extend(api.favorites(1).await.unwrap());
        posts.extend(api.profile_statuses(123, 1).await.unwrap());
        posts
    }

    #[tokio::test]
    async fn test_create_post_table() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let result = create_post_table(&pool).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_conversion() {
        let posts = create_test_posts().await;
        for post in posts {
            let internal_post: PostInternal = post.clone().try_into().unwrap();
            let converted_post: Post = internal_post.try_into().unwrap();

            assert_eq!(post.id, converted_post.id);
            assert_eq!(post.text, converted_post.text);
            assert_eq!(post.pic_ids, converted_post.pic_ids);
            assert_eq!(post.geo, converted_post.geo);
            assert_eq!(post.pic_infos, converted_post.pic_infos);
        }
    }

    #[tokio::test]
    async fn test_save_and_get_post() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for post in posts {
            let internal_post: PostInternal = post.clone().try_into().unwrap();

            save_post(&db, &internal_post, false).await.unwrap();

            let fetched_post = get_post(&db, post.id).await.unwrap().unwrap();
            assert_eq!(internal_post, fetched_post);
        }
    }

    #[tokio::test]
    async fn test_save_post_overwrite() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for post in posts {
            let mut internal_post: PostInternal = post.try_into().unwrap();

            save_post(&db, &internal_post, false).await.unwrap();

            internal_post.text = "updated text".to_string();
            save_post(&db, &internal_post, true).await.unwrap();

            let fetched_post = get_post(&db, internal_post.id).await.unwrap().unwrap();
            assert_eq!(fetched_post.text, "updated text");
        }
    }

    #[tokio::test]
    async fn test_get_favorites() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let posts = get_favorites(&db, 2, 1, false).await.unwrap();
        assert_eq!(posts.len(), 2);

        let posts_rev = get_favorites(&db, 2, 1, true).await.unwrap();
        assert_eq!(posts_rev.len(), 2);
    }

    #[tokio::test]
    async fn test_mark_post_favorited_and_unfavorited() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal_post, false).await.unwrap();

            mark_post_favorited(&db, internal_post.id).await.unwrap();
            let fetched = get_post(&db, internal_post.id).await.unwrap().unwrap();
            assert!(fetched.favorited);

            mark_post_unfavorited(&db, internal_post.id).await.unwrap();
            let fetched = get_post(&db, internal_post.id).await.unwrap().unwrap();
            assert!(fetched.unfavorited);
        }
    }

    #[tokio::test]
    async fn test_get_favorited_sum() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        let mut favorited_count = 0;
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            if internal_post.favorited {
                favorited_count += 1;
            }
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let sum = get_favorited_sum(&db).await.unwrap();
        assert_eq!(sum, favorited_count);
    }

    #[tokio::test]
    async fn test_get_posts_id_to_unfavorite() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        let mut unfavorite_ids = Vec::new();
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            if internal_post.favorited && !internal_post.unfavorited {
                unfavorite_ids.push(internal_post.id);
            }
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let ids = get_posts_id_to_unfavorite(&db).await.unwrap();
        unfavorite_ids.sort();
        let mut ids_sorted = ids;
        ids_sorted.sort();
        assert_eq!(ids_sorted, unfavorite_ids);
    }

    #[tokio::test]
    async fn test_get_posts() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for post in posts.clone() {
            let internal_post: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let fetched_posts = get_posts(&db, 5, 0, false).await.unwrap();
        assert_eq!(fetched_posts.len(), 5);
        assert_eq!(
            fetched_posts[0].id,
            posts.iter().map(|p| p.id).max().unwrap()
        );

        let fetched_posts_rev = get_posts(&db, 5, 0, true).await.unwrap();
        assert_eq!(fetched_posts_rev.len(), 5);
        assert_eq!(
            fetched_posts_rev[0].id,
            posts.iter().map(|p| p.id).min().unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_ones_posts() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        let uid = posts
            .iter()
            .find_map(|p| p.user.as_ref().map(|u| u.id))
            .unwrap();
        let ones_posts_num = posts
            .iter()
            .filter(|p| p.user.is_some() && p.user.as_ref().unwrap().id == uid)
            .count();
        for post in posts.clone() {
            let internal_post: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let fetched_posts = get_ones_posts(&db, uid, ones_posts_num as u32, 0, false)
            .await
            .unwrap();
        assert_eq!(fetched_posts.len(), ones_posts_num);

        let fetched_posts_rev = get_ones_posts(&db, uid, ones_posts_num as u32, 0, true)
            .await
            .unwrap();
        assert_eq!(fetched_posts_rev.len(), ones_posts_num);
    }
}
