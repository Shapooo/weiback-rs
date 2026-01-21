use chrono::DateTime;
use log::debug;
use serde_json::{Value, from_value, to_value};
use sqlx::{FromRow, Sqlite, SqlitePool};

use crate::error::{Error, Result};
use crate::models::Post;

#[derive(Debug, Clone, PartialEq, FromRow, serde::Serialize, serde::Deserialize)]
pub struct PostInternal {
    pub attitudes_count: Option<i64>,
    pub attitudes_status: i64,
    pub comments_count: Option<i64>,
    pub created_at: String,
    pub deleted: bool,
    pub edit_count: Option<i64>,
    pub favorited: bool,
    pub geo: Option<Value>,
    pub id: i64,
    pub mblogid: String,
    pub mix_media_ids: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub page_info: Option<Value>,
    pub pic_ids: Option<Value>,
    pub pic_infos: Option<Value>,
    pub pic_num: Option<i64>,
    pub region_name: Option<String>,
    pub reposts_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub retweeted_id: Option<i64>,
    pub source: Option<String>,
    pub text: String,
    pub uid: Option<i64>,
    pub url_struct: Option<Value>,
}

impl TryFrom<Post> for PostInternal {
    type Error = Error;
    fn try_from(post: Post) -> Result<Self> {
        Ok(Self {
            attitudes_count: post.attitudes_count,
            attitudes_status: post.attitudes_status,
            comments_count: post.comments_count,
            created_at: post.created_at.to_rfc3339(),
            deleted: post.deleted,
            edit_count: post.edit_count,
            favorited: post.favorited,
            geo: post.geo,
            id: post.id,
            mblogid: post.mblogid,
            mix_media_ids: post.mix_media_ids.map(to_value).transpose()?,
            mix_media_info: post.mix_media_info.map(to_value).transpose()?,
            page_info: post.page_info.map(to_value).transpose()?,
            pic_ids: post.pic_ids.map(|v| to_value(&v)).transpose()?,
            pic_infos: post.pic_infos.map(|h| to_value(&h)).transpose()?,
            pic_num: post.pic_num,
            region_name: post.region_name,
            reposts_count: post.reposts_count,
            repost_type: post.repost_type,
            retweeted_id: post.retweeted_status.map(|r| r.id),
            source: post.source,
            text: post.text,
            uid: post.user.map(|u| u.id),
            url_struct: post.url_struct.map(to_value).transpose()?,
        })
    }
}

impl TryInto<Post> for PostInternal {
    type Error = Error;
    fn try_into(self) -> Result<Post> {
        Ok(Post {
            attitudes_count: self.attitudes_count,
            attitudes_status: self.attitudes_status,
            comments_count: self.comments_count,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?,
            deleted: self.deleted,
            edit_count: self.edit_count,
            favorited: self.favorited,
            geo: self.geo,
            id: self.id,
            mblogid: self.mblogid,
            mix_media_ids: self.mix_media_ids.map(from_value).transpose()?,
            mix_media_info: self.mix_media_info.map(from_value).transpose()?,
            page_info: self.page_info.map(from_value).transpose()?,
            pic_ids: self.pic_ids.map(from_value).transpose()?,
            pic_infos: self.pic_infos.map(from_value).transpose()?,
            pic_num: self.pic_num,
            region_name: self.region_name,
            reposts_count: self.reposts_count,
            repost_type: self.repost_type,
            retweeted_status: None,
            source: self.source,
            text: self.text,
            url_struct: self.url_struct.map(from_value).transpose()?,
            user: None,
        })
    }
}

#[allow(unused)]
async fn check_unfavorited(db: &SqlitePool, id: i64) -> Result<Option<bool>> {
    Ok(sqlx::query_scalar::<Sqlite, (bool)>(
        "SELECT unfavorited FROM favorited_posts WHERE id = ?;",
    )
    .bind(id)
    .fetch_optional(db)
    .await?)
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
        r#"SELECT
    *
FROM
    posts
WHERE
    id IN (
        SELECT
            id
        FROM
            favorited_posts
        ORDER BY
            id
        LIMIT
            ?
        OFFSET
            ?
    );"#
    } else {
        r#"SELECT
    *
FROM
    posts
WHERE
    id IN (
        SELECT
            id
        FROM
            favorited_posts
        ORDER BY
            id DESC
        LIMIT
            ?
        OFFSET
            ?
    );"#
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
        "SELECT * FROM posts ORDER BY id LIMIT ? OFFSET ?;"
    } else {
        "SELECT * FROM posts ORDER BY id DESC LIMIT ? OFFSET ?;"
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
        "SELECT * FROM posts WHERE uid = ? ORDER BY id LIMIT ? OFFSET ?;"
    } else {
        "SELECT * FROM posts WHERE uid = ? ORDER BY id DESC LIMIT ? OFFSET ?;"
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
            r#"INSERT
OR {} INTO posts (
    attitudes_count,
    attitudes_status,
    comments_count,
    created_at,
    deleted,
    edit_count,
    favorited,
    geo,
    id,
    mblogid,
    mix_media_ids,
    mix_media_info,
    page_info,
    pic_ids,
    pic_infos,
    pic_num,
    region_name,
    reposts_count,
    repost_type,
    retweeted_id,
    source,
    text,
    uid,
    url_struct
)
VALUES
    (
        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?
    );"#,
            if overwrite { "REPLACE" } else { "IGNORE" }
        )
        .as_str(),
    )
    .bind(post.attitudes_count)
    .bind(post.attitudes_status)
    .bind(post.comments_count)
    .bind(&post.created_at)
    .bind(post.deleted)
    .bind(post.edit_count)
    .bind(post.favorited)
    .bind(&post.geo)
    .bind(post.id)
    .bind(&post.mblogid)
    .bind(&post.mix_media_ids)
    .bind(&post.mix_media_info)
    .bind(&post.page_info)
    .bind(&post.pic_ids)
    .bind(&post.pic_infos)
    .bind(post.pic_num)
    .bind(&post.region_name)
    .bind(post.reposts_count)
    .bind(post.repost_type)
    .bind(post.retweeted_id)
    .bind(&post.source)
    .bind(&post.text)
    .bind(post.uid)
    .bind(&post.url_struct)
    .execute(db)
    .await?;
    if post.favorited {
        mark_post_favorited(db, post.id).await?;
    }
    Ok(())
}

pub async fn mark_post_unfavorited(db: &SqlitePool, id: i64) -> Result<()> {
    debug!("unfav post {id} in db");
    sqlx::query("UPDATE favorited_posts SET unfavorited = true WHERE id = ?;")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn mark_post_favorited(db: &SqlitePool, id: i64) -> Result<()> {
    debug!("mark favorited post {id} in db");
    sqlx::query(
        r#"INSERT
OR REPLACE INTO favorited_posts (id, unfavorited)
VALUES
    (?, ?);"#,
    )
    .bind(id)
    .bind(false)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_favorited_sum(db: &SqlitePool) -> Result<u32> {
    Ok(
        sqlx::query_scalar::<Sqlite, u32>("SELECT COUNT(1) FROM favorited_posts;")
            .fetch_one(db)
            .await?,
    )
}

pub async fn get_posts_id_to_unfavorite(db: &SqlitePool) -> Result<Vec<i64>> {
    debug!("query all posts to unfavorite");
    Ok(sqlx::query_scalar::<Sqlite, i64>(
        "SELECT id FROM favorited_posts WHERE unfavorited == false;",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .collect())
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;
    use std::path::Path;

    use sqlx::SqlitePool;

    use super::*;
    use crate::api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    async fn create_test_posts() -> Vec<Post> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).unwrap();
        let favs = serde_json::from_str::<FavoritesSucc>(s.as_str()).unwrap();
        let mut favs: Vec<Post> = favs
            .favorites
            .into_iter()
            .map(|p| p.status.try_into())
            .collect::<Result<_>>()
            .unwrap();
        let profile_statuses =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/profile_statuses.json");
        let statuses = serde_json::from_str::<ProfileStatusesSucc>(
            read_to_string(profile_statuses).unwrap().as_str(),
        )
        .unwrap();
        let statuses: Vec<Post> = statuses
            .cards
            .into_iter()
            .filter_map(|c| c.mblog.map(|p| p.try_into()))
            .collect::<Result<_>>()
            .unwrap();
        favs.extend(statuses);
        favs
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
        }
    }

    #[tokio::test]
    async fn test_save_and_get_post() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        for mut post in posts {
            let internal_post: PostInternal = post.clone().try_into().unwrap();
            save_post(&db, &internal_post, false).await.unwrap();

            let fetched_post = get_post(&db, post.id)
                .await
                .unwrap()
                .unwrap()
                .try_into()
                .unwrap();

            post.user.take();
            post.retweeted_status.take();
            assert_eq!(post, fetched_post);
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
            let unfavorited = get_post(&db, internal_post.id).await.unwrap();
            assert!(unfavorited.is_some());

            mark_post_unfavorited(&db, internal_post.id).await.unwrap();
            let unfavorited = check_unfavorited(&db, internal_post.id).await.unwrap();
            assert!(unfavorited.unwrap());
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
        let mut ids_to_unfavorite = Vec::new();
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            if internal_post.favorited {
                ids_to_unfavorite.push(internal_post.id);
            }
            save_post(&db, &internal_post, false).await.unwrap();
        }

        let ids = get_posts_id_to_unfavorite(&db).await.unwrap();
        ids_to_unfavorite.sort();
        let mut ids_sorted = ids;
        ids_sorted.sort();
        assert_eq!(ids_sorted, ids_to_unfavorite);
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
