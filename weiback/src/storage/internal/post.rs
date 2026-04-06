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
use sea_query::{Asterisk, Expr, Func, Iden, OnConflict, Order, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};
use sqlx::{Acquire, Executor, FromRow, Sqlite};
use tracing::{debug, error};

use crate::core::task::{PostQuery, SearchTerm};
use crate::error::{Error, Result};
use crate::models::Post;

#[derive(Iden)]
#[iden = "posts"]
enum PostIden {
    Table,
    AttitudesCount,
    AttitudesStatus,
    CommentsCount,
    CreatedAt,
    Deleted,
    EditCount,
    Favorited,
    Geo,
    Id,
    Mblogid,
    MixMediaIds,
    MixMediaInfo,
    PageInfo,
    PicIds,
    PicInfos,
    PicNum,
    RegionName,
    RepostsCount,
    RepostType,
    RetweetedId,
    Source,
    TagStruct,
    Text,
    Uid,
    UrlStruct,
}

#[derive(Iden)]
#[iden = "favorited_posts"]
enum FavoritedPostIden {
    Table,
    Id,
    Unfavorited,
}

#[derive(Iden)]
#[iden = "posts_fts"]
enum PostFtsIden {
    Table,
    #[allow(unused)]
    Text,
}

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
            idstr: self.id.to_string(),
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
    let (sql, values) = Query::select()
        .column(FavoritedPostIden::Unfavorited)
        .from(FavoritedPostIden::Table)
        .and_where(Expr::col(FavoritedPostIden::Id).eq(id))
        .build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_scalar_with::<Sqlite, bool, _>(&sql, values)
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
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(PostIden::Table)
        .and_where(Expr::col(PostIden::Id).eq(id))
        .build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_as_with::<Sqlite, PostInternal, _>(&sql, values)
        .fetch_optional(executor)
        .await?)
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
pub async fn save_post<'c, A>(acquirer: A, post: &PostInternal) -> Result<()>
where
    A: Acquire<'c, Database = Sqlite>,
{
    use serde_json::to_string;
    let mut conn = acquirer.acquire().await?;
    let (sql, values) = Query::insert()
        .into_table(PostIden::Table)
        .columns([
            PostIden::AttitudesCount,
            PostIden::AttitudesStatus,
            PostIden::CommentsCount,
            PostIden::CreatedAt,
            PostIden::Deleted,
            PostIden::EditCount,
            PostIden::Favorited,
            PostIden::Geo,
            PostIden::Id,
            PostIden::Mblogid,
            PostIden::MixMediaIds,
            PostIden::MixMediaInfo,
            PostIden::PageInfo,
            PostIden::PicIds,
            PostIden::PicInfos,
            PostIden::PicNum,
            PostIden::RegionName,
            PostIden::RepostsCount,
            PostIden::RepostType,
            PostIden::RetweetedId,
            PostIden::Source,
            PostIden::TagStruct,
            PostIden::Text,
            PostIden::Uid,
            PostIden::UrlStruct,
        ])
        .values([
            post.attitudes_count.into(),
            post.attitudes_status.into(),
            post.comments_count.into(),
            post.created_at.clone().into(),
            post.deleted.into(),
            post.edit_count.into(),
            post.favorited.into(),
            post.geo.as_ref().map(to_string).transpose()?.into(),
            post.id.into(),
            post.mblogid.clone().into(),
            post.mix_media_ids
                .as_ref()
                .map(to_string)
                .transpose()?
                .into(),
            post.mix_media_info
                .as_ref()
                .map(to_string)
                .transpose()?
                .into(),
            post.page_info.as_ref().map(to_string).transpose()?.into(),
            post.pic_ids.as_ref().map(to_string).transpose()?.into(),
            post.pic_infos.as_ref().map(to_string).transpose()?.into(),
            post.pic_num.into(),
            post.region_name.clone().into(),
            post.reposts_count.into(),
            post.repost_type.into(),
            post.retweeted_id.into(),
            post.source.clone().into(),
            post.tag_struct.as_ref().map(to_string).transpose()?.into(),
            post.text.clone().into(),
            post.uid.into(),
            post.url_struct.as_ref().map(to_string).transpose()?.into(),
        ])?
        .on_conflict(
            OnConflict::column(PostIden::Id)
                .update_columns([
                    PostIden::AttitudesCount,
                    PostIden::AttitudesStatus,
                    PostIden::CommentsCount,
                    PostIden::CreatedAt,
                    PostIden::Deleted,
                    PostIden::EditCount,
                    PostIden::Favorited,
                    PostIden::Geo,
                    PostIden::Mblogid,
                    PostIden::MixMediaIds,
                    PostIden::MixMediaInfo,
                    PostIden::PageInfo,
                    PostIden::PicIds,
                    PostIden::PicInfos,
                    PostIden::PicNum,
                    PostIden::RegionName,
                    PostIden::RepostsCount,
                    PostIden::RepostType,
                    PostIden::RetweetedId,
                    PostIden::Source,
                    PostIden::TagStruct,
                    PostIden::Text,
                    PostIden::Uid,
                    PostIden::UrlStruct,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(&mut *conn).await?;

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
    let (sql, values) = Query::update()
        .table(FavoritedPostIden::Table)
        .values([(FavoritedPostIden::Unfavorited, true.into())])
        .and_where(Expr::col(FavoritedPostIden::Id).eq(id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(executor).await?;
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
    let (sql, values) = Query::insert()
        .into_table(FavoritedPostIden::Table)
        .columns([FavoritedPostIden::Id, FavoritedPostIden::Unfavorited])
        .values([id.into(), false.into()])?
        .on_conflict(
            OnConflict::column(FavoritedPostIden::Id)
                .update_column(FavoritedPostIden::Unfavorited)
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(executor).await?;
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
    let (sql, values) = Query::select()
        .column(FavoritedPostIden::Id)
        .from(FavoritedPostIden::Table)
        .and_where(Expr::col(FavoritedPostIden::Unfavorited).eq(false))
        .build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_scalar_with::<Sqlite, i64, _>(&sql, values)
        .fetch_all(executor)
        .await?)
}

/// Retrieves the IDs of posts that are invalid.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `clean_retweeted_invalid` - Whether to include posts that are valid themselves but their retweeted content is invalid.
///
/// # Returns
///
/// A `Result` containing a `Vec<i64>` of invalid post IDs.
pub async fn get_invalid_posts_ids<'e, E>(
    executor: E,
    clean_retweeted_invalid: bool,
) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    debug!(
        "query invalid posts, clean_retweeted_invalid: {}",
        clean_retweeted_invalid
    );

    let mut query = Query::select();
    query.column(PostIden::Id).from(PostIden::Table);

    if clean_retweeted_invalid {
        // SELECT id FROM posts WHERE uid IS NULL
        // delete_post will alse delete all retweets of the post
        query.and_where(Expr::col(PostIden::Uid).is_null());
    } else {
        // SELECT id FROM posts WHERE uid IS NULL AND id NOT IN (SELECT retweeted_id FROM posts WHERE retweeted_id IS NOT NULL)
        query
            .and_where(Expr::col(PostIden::Uid).is_null())
            .and_where(
                Expr::col(PostIden::Id).not_in_subquery(
                    Query::select()
                        .column(PostIden::RetweetedId)
                        .from(PostIden::Table)
                        .and_where(Expr::col(PostIden::RetweetedId).is_not_null())
                        .take(),
                ),
            );
    }

    let (sql, values) = query.build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_scalar_with::<Sqlite, i64, _>(&sql, values)
        .fetch_all(executor)
        .await?)
}

/// Retrieves the IDs of posts that retweet a given original post.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the original(retweeted) post.
///
/// # Returns
///
/// A `Result` containing a `Vec<i64>` of post IDs that are retweets of the given post.
pub async fn get_retweet_ids<'e, E>(executor: E, id: i64) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(PostIden::Id)
        .from(PostIden::Table)
        .and_where(Expr::col(PostIden::RetweetedId).eq(id))
        .build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_scalar_with::<Sqlite, i64, _>(&sql, values)
        .fetch_all(executor)
        .await?)
}

/// Checks if a post has any children (is being retweeted by other posts).
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The ID of the post to check.
///
/// # Returns
///
/// A `Result` containing `true` if the post has children, `false` otherwise.
pub async fn has_retweets<'e, E>(executor: E, id: i64) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(PostIden::Id)
        .from(PostIden::Table)
        .and_where(Expr::col(PostIden::RetweetedId).eq(id))
        .limit(1)
        .build_sqlx(SqliteQueryBuilder);
    Ok(sqlx::query_scalar_with::<Sqlite, i64, _>(&sql, values)
        .fetch_optional(executor)
        .await?
        .is_some())
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
    batch_delete_posts(acquirer, &[id]).await
}

/// Deletes multiple posts and their corresponding entries in `favorited_posts` table from the database.
///
/// # Arguments
///
/// * `acquirer` - A database acquirer.
/// * `ids` - A slice of IDs of the posts to delete.
///
/// # Returns
///
/// A `Result` indicating success or failure.
pub async fn batch_delete_posts<'c, A>(acquirer: A, ids: &[i64]) -> Result<()>
where
    A: Acquire<'c, Database = Sqlite>,
{
    if ids.is_empty() {
        return Ok(());
    }
    let mut conn = acquirer.acquire().await?;
    let (sql, values) = Query::delete()
        .from_table(PostIden::Table)
        .and_where(Expr::col(PostIden::Id).is_in(ids.iter().cloned()))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(&mut *conn).await?;

    let (sql, values) = Query::delete()
        .from_table(FavoritedPostIden::Table)
        .and_where(Expr::col(FavoritedPostIden::Id).is_in(ids.iter().cloned()))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(&mut *conn).await?;
    Ok(())
}

/// Builds the common part of a post query based on the given `PostQuery` criteria.
/// This includes joins for FTS and all the `WHERE` clause filters.
fn build_common_query(query: &PostQuery) -> Result<sea_query::SelectStatement> {
    use sea_query::{Alias, JoinType, UnionType};

    let mut posts_query = Query::select();
    posts_query.from(PostIden::Table);

    if let Some(search_term) = query.search_term.as_ref() {
        let fts_query = match search_term {
            SearchTerm::Fuzzy(term) => term.to_owned(),
            SearchTerm::Strict(term) => format!("\"{}\"", term.replace('"', "\"\"")),
        };

        let mut subquery_self = Query::select();
        subquery_self
            .column((PostIden::Table, PostIden::Id))
            .from(PostIden::Table)
            .join(
                JoinType::InnerJoin,
                PostFtsIden::Table,
                Expr::col((PostFtsIden::Table, Alias::new("rowid")))
                    .eq(Expr::col((PostIden::Table, PostIden::Id))),
            )
            .and_where(Expr::cust_with_values(
                "posts_fts.text MATCH ?",
                [fts_query.clone()],
            ));

        let mut subquery_retweet = Query::select();
        subquery_retweet
            .column((PostIden::Table, PostIden::Id))
            .from(PostIden::Table)
            .join(
                JoinType::InnerJoin,
                PostFtsIden::Table,
                Expr::col((PostFtsIden::Table, Alias::new("rowid")))
                    .eq(Expr::col((PostIden::Table, PostIden::RetweetedId))),
            )
            .and_where(Expr::cust_with_values(
                "posts_fts.text MATCH ?",
                [fts_query],
            ));

        subquery_self.union(UnionType::All, subquery_retweet.take());

        posts_query.and_where(
            Expr::col((PostIden::Table, PostIden::Id)).in_subquery(subquery_self.take()),
        );
    }

    if let Some(user_id) = query.user_id {
        posts_query.and_where(Expr::col((PostIden::Table, PostIden::Uid)).eq(user_id));
    }

    if let Some(start_date) = query.start_date {
        let dt = DateTime::from_timestamp(start_date, 0)
            .ok_or_else(|| {
                let msg = "无效的开始时间".to_string();
                error!("{msg}");
                Error::FormatError(msg)
            })?
            .to_rfc3339();
        posts_query.and_where(Expr::col((PostIden::Table, PostIden::CreatedAt)).gte(dt));
    }

    if let Some(end_date) = query.end_date {
        let dt = DateTime::from_timestamp(end_date, 0)
            .ok_or_else(|| {
                let msg = "无效的结束时间".to_string();
                error!("{msg}");
                Error::FormatError(msg)
            })?
            .to_rfc3339();
        posts_query.and_where(Expr::col((PostIden::Table, PostIden::CreatedAt)).lte(dt));
    }

    if query.is_favorited {
        posts_query.and_where(
            Expr::col((PostIden::Table, PostIden::Id)).in_subquery(
                Query::select()
                    .column(FavoritedPostIden::Id)
                    .from(FavoritedPostIden::Table)
                    .take(),
            ),
        );
    }

    Ok(posts_query)
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
    let mut posts_query = build_common_query(&query)?;

    let mut count_query = posts_query.clone();
    count_query.expr(if query.search_term.is_some() {
        Expr::col((PostIden::Table, PostIden::Id)).count_distinct()
    } else {
        Func::count(1).into()
    });

    let (sql, values) = count_query.build_sqlx(SqliteQueryBuilder);
    let mut conn = acquirer.acquire().await?;
    let total_items: u64 = sqlx::query_scalar_with(&sql, values)
        .fetch_one(&mut *conn)
        .await?;

    let order = if query.reverse_order {
        Order::Asc
    } else {
        Order::Desc
    };
    posts_query
        .column((PostIden::Table, Asterisk))
        .order_by((PostIden::Table, PostIden::Id), order)
        .limit(query.posts_per_page as u64)
        .offset((query.page.saturating_sub(1) * query.posts_per_page) as u64);

    let (sql, values) = posts_query.build_sqlx(SqliteQueryBuilder);
    let posts = sqlx::query_as_with::<Sqlite, PostInternal, _>(&sql, values)
        .fetch_all(&mut *conn)
        .await?;

    Ok((posts, total_items))
}

/// Queries all post IDs from the database based on various criteria, without pagination.
///
/// # Arguments
///
/// * `acquirer` - A database acquirer.
/// * `query` - A `PostQuery` struct specifying the query parameters.
///
/// # Returns
///
/// A `Result` containing a vector of `i64` representing the post IDs.
pub async fn query_all_post_ids<'c, A>(acquirer: A, query: PostQuery) -> Result<Vec<i64>>
where
    A: Acquire<'c, Database = Sqlite>,
{
    let mut posts_query = build_common_query(&query)?;

    posts_query.column((PostIden::Table, PostIden::Id));

    let (sql, values) = posts_query.build_sqlx(SqliteQueryBuilder);
    let mut conn = acquirer.acquire().await?;
    let ids = sqlx::query_scalar_with::<Sqlite, i64, _>(&sql, values)
        .fetch_all(&mut *conn)
        .await?;

    Ok(ids)
}

#[cfg(test)]
mod local_tests {
    use std::collections::HashSet;
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
            save_post(&db, &internal_post).await.unwrap();

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

            save_post(&db, &internal_post).await.unwrap();

            internal_post.text = "updated text".to_string();
            save_post(&db, &internal_post).await.unwrap();

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
            save_post(&db, &internal_post).await.unwrap();
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
            save_post(&db, &internal_post).await.unwrap();

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
        let mut favorited_set = HashSet::new();
        for post in posts {
            let internal_post: PostInternal = post.try_into().unwrap();
            if internal_post.favorited {
                favorited_set.insert(internal_post.id);
            }
            save_post(&db, &internal_post).await.unwrap();
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
        assert_eq!(sum, favorited_set.len() as u64);
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
            save_post(&db, &internal_post).await.unwrap();
        }

        let ids = get_posts_id_to_unfavorite(&db).await.unwrap();
        ids_to_unfavorite.sort();
        ids_to_unfavorite.dedup();
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
            save_post(&db, &internal_post).await.unwrap();
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
        let mut ones_post_ids = posts
            .iter()
            .filter_map(|p| {
                if p.user.is_some() && p.user.as_ref().unwrap().id == uid {
                    Some(p.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        ones_post_ids.sort();
        ones_post_ids.dedup();
        for post in posts.clone() {
            let internal_post: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal_post).await.unwrap();
        }

        let mut query = PostQuery {
            user_id: Some(uid),
            start_date: None,
            end_date: None,
            search_term: None,
            is_favorited: false,
            reverse_order: true,
            page: 1,
            posts_per_page: ones_post_ids.len() as u32,
        };
        let (fetched_posts, sum) = query_posts(&db, query.clone()).await.unwrap();
        let fetched_ids = fetched_posts.into_iter().map(|p| p.id).collect::<Vec<_>>();
        assert_eq!(fetched_ids, ones_post_ids);
        assert_eq!(sum as usize, ones_post_ids.len());

        query.reverse_order = false;
        ones_post_ids.reverse();
        let (fetched_posts_rev, sum) = query_posts(&db, query).await.unwrap();
        let fetched_ids_rev = fetched_posts_rev
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(fetched_ids_rev, ones_post_ids);
        assert_eq!(sum as usize, ones_post_ids.len());
    }

    #[tokio::test]
    async fn test_delete_post() {
        let db = setup_db().await;
        let mut posts = create_test_posts().await;
        let post = posts.remove(0);
        let internal_post: PostInternal = post.try_into().unwrap();
        save_post(&db, &internal_post).await.unwrap();

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
    async fn test_batch_delete_posts() {
        let db = setup_db().await;
        let mut posts = create_test_posts().await;
        let p1 = posts.remove(0);
        let p2 = posts.remove(0);
        let id1 = p1.id;
        let id2 = p2.id;

        save_post(&db, &p1.try_into().unwrap()).await.unwrap();
        save_post(&db, &p2.try_into().unwrap()).await.unwrap();

        assert!(get_post(&db, id1).await.unwrap().is_some());
        assert!(get_post(&db, id2).await.unwrap().is_some());

        batch_delete_posts(&db, &[id1, id2]).await.unwrap();

        assert!(get_post(&db, id1).await.unwrap().is_none());
        assert!(get_post(&db, id2).await.unwrap().is_none());
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

        save_post(&db, &internal_original).await.unwrap();
        save_post(&db, &internal_retweeted).await.unwrap();

        let ids = get_retweet_ids(&db, internal_original.id).await.unwrap();
        assert_eq!(ids, vec![internal_retweeted.id]);
    }

    #[tokio::test]
    async fn test_has_retweets() {
        let db = setup_db().await;
        let posts = create_test_posts().await;

        // Find an original post (no retweet) and a retweeted post
        let original_post = posts.iter().find(|p| p.retweeted_status.is_none()).unwrap();
        let mut retweeted_post = posts
            .iter()
            .find(|p| p.retweeted_status.is_some())
            .unwrap()
            .clone();
        retweeted_post.retweeted_status = Some(Box::new(original_post.clone()));

        let internal_original: PostInternal = original_post.clone().try_into().unwrap();
        let internal_retweeted: PostInternal = retweeted_post.try_into().unwrap();

        save_post(&db, &internal_original).await.unwrap();
        save_post(&db, &internal_retweeted).await.unwrap();

        // Original post has retweets
        assert!(has_retweets(&db, internal_original.id).await.unwrap());
        // Retweeted post has no retweets
        assert!(!has_retweets(&db, internal_retweeted.id).await.unwrap());

        // A post that doesn't exist returns false
        assert!(!has_retweets(&db, 999999).await.unwrap());
    }

    #[tokio::test]
    async fn test_query_posts_search_term() {
        let db = setup_db().await;
        let posts = vec![
            Post {
                id: 1,
                text: "hello world".to_string(),
                ..Default::default()
            },
            Post {
                id: 2,
                text: "rust 编程语言".to_string(),
                ..Default::default()
            },
            Post {
                id: 3,
                text: "你好 rust".to_string(),
                ..Default::default()
            },
            Post {
                id: 4,
                text: "微博备份工具".to_string(),
                ..Default::default()
            },
            Post {
                id: 5,
                text: "备份很重要".to_string(),
                ..Default::default()
            },
        ];

        for post in posts {
            let internal: PostInternal = post.try_into().unwrap();
            save_post(&db, &internal).await.unwrap();
        }

        let mut query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            search_term: Some(SearchTerm::Fuzzy("hello".to_string())),
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: 10,
        };

        // Fuzzy search "hello"
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(results[0].id, 1);

        // 中文模糊搜索 "备份工" (3个字符)
        query.search_term = Some(SearchTerm::Fuzzy("备份工".to_string()));
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(results[0].id, 4);

        // 中文模糊搜索 "备份很" (3个字符)
        query.search_term = Some(SearchTerm::Fuzzy("备份很".to_string()));
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(results[0].id, 5);

        // 中英混合模糊搜索 "你好 rust"
        query.search_term = Some(SearchTerm::Fuzzy("你好 rust".to_string()));
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 2);
        assert_eq!(results[0].id, 3);

        // 纯中文模糊搜索 "微博备" (3个字符)
        query.search_term = Some(SearchTerm::Fuzzy("微博备".to_string()));
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(results[0].id, 4);

        // 中英混合精确搜索 "rust 编程语言"
        query.search_term = Some(SearchTerm::Strict("rust 编程语言".to_string()));
        let (results, total) = query_posts(&db, query.clone()).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(results[0].id, 2);

        // Fuzzy search with no matches
        query.search_term = Some(SearchTerm::Fuzzy("nomatch".to_string()));
        let (results, total) = query_posts(&db, query).await.unwrap();
        assert_eq!(total, 0);
        assert_eq!(results.len(), 0);
    }
}
