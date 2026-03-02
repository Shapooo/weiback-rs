//! This module handles the storage and retrieval of `Post` data, along with related
//! operations such as marking posts as favorited/unfavorited, retrieving retweeted posts,
//! and querying posts based on various criteria.
//!
//! It interacts with the `posts` and `favorited_posts` tables in the database.
//!
//! # Table Structure: `posts`
//!
//! | Column             | Type      | Description                                       |
//! |--------------------|-----------|---------------------------------------------------|
//! | `attitudes_count`  | `INTEGER` | Number of attitudes (likes) for the post.         |
//! | `attitudes_status` | `INTEGER` | Status of attitudes.                              |
//! | `comments_count`   | `INTEGER` | Number of comments on the post.                   |
//! | `created_at`       | `TEXT`    | Timestamp of post creation (RFC3339 format).      |
//! | `deleted`          | `BOOLEAN` | Whether the post has been marked as deleted.      |
//! | `edit_count`       | `INTEGER` | Number of times the post has been edited.         |
//! | `favorited`        | `BOOLEAN` | Whether the post is marked as favorited.          |
//! | `geo`              | `JSON`    | Geographical information as JSON.                 |
//! | `id`               | `INTEGER` | Unique identifier for the post. **Primary Key.**  |
//! | `mblogid`          | `TEXT`    | Microblog ID.                                     |
//! | `mix_media_ids`    | `JSON`    | Mixed media IDs as JSON array.                    |
//! | `mix_media_info`   | `JSON`    | Mixed media information as JSON.                  |
//! | `page_info`        | `JSON`    | Page-specific information as JSON.                |
//! | `pic_ids`          | `JSON`    | Picture IDs as JSON array.                        |
//! | `pic_infos`        | `JSON`    | Picture information as JSON object.               |
//! | `pic_num`          | `INTEGER` | Number of pictures in the post.                   |
//! | `region_name`      | `TEXT`    | Region name of the post.                          |
//! | `reposts_count`    | `INTEGER` | Number of reposts.                                |
//! | `repost_type`      | `INTEGER` | Type of repost.                                   |
//! | `retweeted_id`     | `INTEGER` | ID of the original post if this is a retweet.     |
//! | `source`           | `TEXT`    | Source of the post (e.g., "iPhone client").       |
//! | `tag_struct`       | `JSON`    | Tag structure as JSON.                            |
//! | `text`             | `TEXT`    | The main text content of the post.                |
//! | `uid`              | `INTEGER` | User ID of the post author.                       |
//! | `url_struct`       | `JSON`    | URL structure as JSON.                            |
//!
//! The `id` column serves as the primary key for uniqueness in the `posts` table.
//!
//! # Table Structure: `favorited_posts`
//!
//! | Column        | Type      | Description                                       |
//! |---------------|-----------|---------------------------------------------------|
//! | `id`          | `INTEGER` | The ID of the favorited post. **Primary Key.**    |
//! | `unfavorited` | `BOOLEAN` | True if the post has been unfavorited.            |

use chrono::DateTime;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};
use sqlx::{Acquire, Executor, FromRow, Sqlite};

use crate::core::task::{PostQuery, SearchTerm};
use crate::error::{Error, Result};
use crate::models::Post;

/// Represents the internal database structure for a post.
/// This struct is used for direct interaction with the `posts` table.
#[derive(Debug, Clone, PartialEq, FromRow, Serialize, Deserialize)]
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
    pub tag_struct: Option<Value>,
    pub text: String,
    pub uid: Option<i64>,
    pub url_struct: Option<Value>,
}

impl TryFrom<Post> for PostInternal {
    type Error = Error;
    /// Tries to convert a `Post` model into a `PostInternal` database representation.
    ///
    /// **Note:** This is a lossy conversion for internal use. The `retweeted_status` field
    /// is not fully preserved; only its ID is stored in `retweeted_id`. The `user` field's
    /// ID is stored in `uid`.
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
            tag_struct: post.tag_struct.map(|t| to_value(&t)).transpose()?,
            text: post.text,
            uid: post.user.map(|u| u.id),
            url_struct: post.url_struct.map(to_value).transpose()?,
        })
    }
}

impl TryInto<Post> for PostInternal {
    type Error = Error;
    /// Tries to convert a `PostInternal` database representation into a `Post` model.
    /// This conversion can fail due to malformed date strings or invalid JSON data.
    ///
    /// **Note:** This is a lossy conversion for internal use. The `retweeted_status` and `user`
    /// fields are always `None` after conversion. The caller is responsible for populating
    /// these fields by fetching them separately from the database if needed.
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
            tag_struct: self.tag_struct.map(from_value).transpose()?,
            text: self.text,
            url_struct: self.url_struct.map(from_value).transpose()?,
            user: None,
        })
    }
}

/// Checks the unfavorited status of a post from the `favorited_posts` table.
/// This function is primarily for internal use and testing.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the post to check.
///
/// # Returns
///
/// A `Result` containing `Option<bool>`. `Some(true)` if unfavorited, `Some(false)` if favorited, `None` if not found.
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

/// Retrieves a single post by its ID.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The unique identifier of the post.
///
/// # Returns
///
/// A `Result` containing an `Option<PostInternal>`. `Some(PostInternal)` if the post is found, `None` otherwise.
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

/// Saves a post's data into the database.
///
/// This function can either insert a new post or replace an existing one based on the `overwrite` flag.
/// It also handles marking the post as favorited if `post.favorited` is true.
///
/// # Arguments
///
/// * `acquirer` - A database acquirer (e.g., `SqlitePool` or `&mut SqliteConnection`).
/// * `post` - The `PostInternal` object to save.
/// * `overwrite` - If `true`, existing posts with the same ID will be replaced. If `false`, they will be ignored.
///
/// # Returns
///
/// A `Result` indicating success or failure.
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
    tag_struct,
    text,
    uid,
    url_struct
)
VALUES
    (
        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?, ?
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
    .bind(&post.tag_struct)
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

/// Marks a post as unfavorited in the `favorited_posts` table.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the post to mark as unfavorited.
///
/// # Returns
///
/// A `Result` indicating success or failure.
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

/// Marks a post as favorited in the `favorited_posts` table.
/// If the post already exists in the table, its unfavorited status will be set to `false`.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the post to mark as favorited.
///
/// # Returns
///
/// A `Result` indicating success or failure.
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

/// Retrieves a list of post IDs that are marked as favorited but not yet unfavorited in the `favorited_posts` table.
///
/// # Arguments
///
/// * `executor` - A database executor.
///
/// # Returns
///
/// A `Result` containing a `Vec<i64>` of post IDs that need to be unfavorited.
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

/// Retrieves the IDs of posts that retweeted a given original post.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the original post.
///
/// # Returns
///
/// A `Result` containing a `Vec<i64>` of post IDs that are retweets of the given post.
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

/// Deletes a post and its corresponding entry in `favorited_posts` table from the database.
///
/// # Arguments
///
/// * `acquirer` - A database acquirer.
/// * `id` - The ID of the post to delete.
///
/// # Returns
///
/// A `Result` indicating success or failure.
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

/// Queries posts from the database based on various criteria.
///
/// This function supports filtering by user ID, date range, search terms, and favorited status.
/// It also handles pagination and ordering.
///
/// # Arguments
///
/// * `acquirer` - A database acquirer.
/// * `query` - A `PostQuery` struct specifying the query parameters.
///
/// # Returns
///
/// A `Result` containing a tuple of `(Vec<PostInternal>, u64)`. The vector contains the
/// matching posts, and `u64` is the total count of items matching the query without pagination.
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

    let (from_clause, select_posts, select_count, order_by) = if query.search_term.is_some() {
        where_conditions.push("f.text MATCH ?");
        (
            "FROM posts p INNER JOIN posts_fts f ON (f.rowid = p.id OR f.rowid = p.retweeted_id)",
            "SELECT DISTINCT p.*",
            "SELECT COUNT(DISTINCT p.id)",
            "p.id",
        )
    } else {
        ("FROM posts", "SELECT *", "SELECT COUNT(*)", "id")
    };

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    let count_sql = format!("{} {} {}", select_count, from_clause, where_clause);
    let mut count_query = sqlx::query_scalar(&count_sql);

    let order = if query.reverse_order { "ASC" } else { "DESC" };
    let posts_sql = format!(
        "{} {} {} ORDER BY {} {} LIMIT ? OFFSET ?",
        select_posts, from_clause, where_clause, order_by, order,
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
    if let Some(search_term) = query.search_term {
        let fts_query = match search_term {
            SearchTerm::Fuzzy(term) => term
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" AND "),
            SearchTerm::Strict(term) => format!("\"{}\"", term.replace('"', "\"\"")),
        };
        count_query = count_query.bind(fts_query.clone());
        posts_query = posts_query.bind(fts_query);
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
    use std::path::Path;

    use sqlx::SqlitePool;
    use tokio::fs::read_to_string;

    use super::*;
    use crate::api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    async fn create_test_posts() -> Vec<Post> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).await.unwrap();
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
            read_to_string(profile_statuses).await.unwrap().as_str(),
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
            search_term: None,
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
            search_term: None,
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
            search_term: None,
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
            search_term: None,
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

    #[tokio::test]
    async fn test_delete_post() {
        let db = setup_db().await;
        let mut posts = create_test_posts().await;
        let post = posts.remove(0);
        let internal_post: PostInternal = post.try_into().unwrap();
        save_post(&db, &internal_post, false).await.unwrap();

        assert!(get_post(&db, internal_post.id).await.unwrap().is_some());
        if internal_post.favorited {
            assert!(
                check_unfavorited(&db, internal_post.id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }

        delete_post(&db, internal_post.id).await.unwrap();

        assert!(get_post(&db, internal_post.id).await.unwrap().is_none());
        assert!(
            check_unfavorited(&db, internal_post.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_retweeted_posts_id() {
        let db = setup_db().await;
        let posts = create_test_posts().await;
        let original_post = posts.iter().find(|p| p.retweeted_status.is_none()).unwrap();
        let mut retweeted_post = posts
            .iter()
            .find(|p| p.retweeted_status.is_some())
            .unwrap()
            .clone();
        retweeted_post.retweeted_status = Some(Box::new(original_post.clone()));

        let internal_original: PostInternal = original_post.clone().try_into().unwrap();
        let internal_retweeted: PostInternal = retweeted_post.try_into().unwrap();

        save_post(&db, &internal_original, false).await.unwrap();
        save_post(&db, &internal_retweeted, false).await.unwrap();

        let ids = get_retweeted_posts_id(&db, internal_original.id)
            .await
            .unwrap();
        assert_eq!(ids, vec![internal_retweeted.id]);
    }
}
