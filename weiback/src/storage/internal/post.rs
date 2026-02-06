use chrono::DateTime;
use log::debug;
use serde_json::{Value, from_value, to_value};
use sqlx::{Acquire, Executor, FromRow, Sqlite};

use crate::core::task::PostQuery;
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
async fn check_unfavorited<'e, E>(executor: E, id: i64) -> Result<Option<bool>>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query_scalar::<Sqlite, (bool)>(
        "SELECT unfavorited FROM favorited_posts WHERE id = ?;",
    )
    .bind(id)
    .fetch_optional(executor)
    .await?)
}

pub async fn get_post<'e, E>(executor: E, id: i64) -> Result<Option<PostInternal>>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_as::<Sqlite, PostInternal>("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(executor)
            .await?,
    )
}

pub async fn save_post<'c, A>(acquirer: A, post: &PostInternal, overwrite: bool) -> Result<()>
where
    A: Acquire<'c, Database = Sqlite>,
{
    let mut conn = acquirer.acquire().await?;
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
    .execute(&mut *conn)
    .await?;
    if post.favorited {
        mark_post_favorited(&mut *conn, post.id).await?;
    }
    Ok(())
}

pub async fn mark_post_unfavorited<'e, E>(executor: E, id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    debug!("unfav post {id} in db");
    sqlx::query("UPDATE favorited_posts SET unfavorited = true WHERE id = ?;")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn mark_post_favorited<'e, E>(executor: E, id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    debug!("mark favorited post {id} in db");
    sqlx::query(
        r#"INSERT
OR REPLACE INTO favorited_posts (id, unfavorited)
VALUES
    (?, ?);"#,
    )
    .bind(id)
    .bind(false)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_posts_id_to_unfavorite<'e, E>(executor: E) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    debug!("query all posts to unfavorite");
    Ok(sqlx::query_scalar::<Sqlite, i64>(
        "SELECT id FROM favorited_posts WHERE unfavorited == false;",
    )
    .fetch_all(executor)
    .await?
    .into_iter()
    .collect())
}

pub async fn get_retweeted_posts_id<'e, E>(executor: E, id: i64) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar("SELECT id FROM posts WHERE retweeted_id = ?")
        .bind(id)
        .fetch_all(executor)
        .await
        .map_err(Into::into)
}

pub async fn delete_post<'c, A>(acquirer: A, id: i64) -> Result<()>
where
    A: Acquire<'c, Database = Sqlite>,
{
    let mut conn = acquirer.acquire().await?;
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM favorited_posts WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn query_posts<'c, A>(acquirer: A, query: PostQuery) -> Result<(Vec<PostInternal>, u64)>
where
    A: Acquire<'c, Database = Sqlite>,
{
    let mut where_conditions = Vec::new();
    if query.user_id.is_some() {
        where_conditions.push("uid = ?");
    }
    if query.start_date.is_some() {
        where_conditions.push("created_at >= ?");
    }
    if query.end_date.is_some() {
        where_conditions.push("created_at <= ?");
    }
    if query.is_favorited {
        where_conditions.push("id IN (SELECT id FROM favorited_posts)");
    }

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM posts {}", where_clause);
    let mut count_query = sqlx::query_scalar(&count_sql);

    let order = if query.reverse_order { "ASC" } else { "DESC" };
    let posts_sql = format!(
        "SELECT * FROM posts {} ORDER BY id {} LIMIT ? OFFSET ?",
        where_clause, order,
    );
    let mut posts_query = sqlx::query_as(&posts_sql);

    if let Some(user_id) = query.user_id {
        count_query = count_query.bind(user_id);
        posts_query = posts_query.bind(user_id);
    }
    if let Some(start_date) = query.start_date {
        let dt = DateTime::from_timestamp(start_date, 0)
            .unwrap()
            .to_rfc3339();
        count_query = count_query.bind(dt.clone());
        posts_query = posts_query.bind(dt);
    }
    if let Some(end_date) = query.end_date {
        let dt = DateTime::from_timestamp(end_date, 0).unwrap().to_rfc3339();
        count_query = count_query.bind(dt.clone());
        posts_query = posts_query.bind(dt);
    }
    let mut conn = acquirer.acquire().await?;
    let total_items = count_query.fetch_one(&mut *conn).await?;

    let limit = query.posts_per_page;
    let offset = query.page.saturating_sub(1) * limit;
    let posts = posts_query
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;

    Ok((posts, total_items))
}

#[cfg(test)]
mod local_tests {
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

        let mut query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            is_favorited: true,
            reverse_order: false,
            page: 1,
            posts_per_page: 2,
        };
        let (posts, _sum) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(posts.len(), 2);

        query.reverse_order = true;
        let (posts_rev, _sum) = query_posts(&db, query).await.unwrap();
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

        let query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            is_favorited: true,
            reverse_order: false,
            page: 1,
            posts_per_page: 2,
        };
        let (_, sum) = query_posts(&db, query).await.unwrap();
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

        let mut query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: 5,
        };
        let (fetched_posts, _sum) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(fetched_posts.len(), 5);
        assert_eq!(
            fetched_posts[0].id,
            posts.iter().map(|p| p.id).max().unwrap()
        );

        query.reverse_order = true;
        let (fetched_posts_rev, _sum) = query_posts(&db, query).await.unwrap();
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

        let mut query = PostQuery {
            user_id: Some(uid),
            start_date: None,
            end_date: None,
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: ones_posts_num as u32,
        };
        let (fetched_posts, sum) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(fetched_posts.len(), ones_posts_num);
        assert_eq!(sum as usize, ones_posts_num);

        query.reverse_order = true;
        let (fetched_posts_rev, sum) = query_posts(&db, query).await.unwrap();
        assert_eq!(fetched_posts_rev.len(), ones_posts_num);
        assert_eq!(sum as usize, ones_posts_num);
    }
}
