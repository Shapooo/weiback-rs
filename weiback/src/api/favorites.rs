//! This module provides an API for interacting with Weibo's favorites (collections) functionality.
//!
//! It includes methods to retrieve a user's favorited posts and to destroy (unfavorite) a post.
//! The module handles the deserialization of API responses into internal `PostInternal` models.
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use serde::Deserialize;
use tracing::{debug, error, info};
use weibosdk_rs::http_client::{HttpClient, HttpResponse};

use super::ApiClientImpl;
use super::internal::post::PostInternal;
use crate::{
    error::{Error, Result},
    models::{Post, err_response::ErrResponse},
};

/// Represents a single favorited post from the API response.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FavoritesPost {
    pub status: PostInternal,
}

/// An enum representing the possible responses from the favorites API endpoint,
/// which can either be a successful data payload or an error.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum FavoritesResponse {
    Succ(FavoritesSucc),
    Fail(ErrResponse),
}

/// Represents the successful response structure for fetching favorites.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FavoritesSucc {
    pub favorites: Vec<FavoritesPost>,
    #[serde(default)]
    #[allow(unused)]
    pub total_number: i32,
}

impl TryFrom<FavoritesResponse> for Vec<PostInternal> {
    type Error = Error;
    /// Tries to convert a `FavoritesResponse` into a vector of `PostInternal` objects.
    ///
    /// This handles both successful API responses and error responses.
    ///
    /// # Arguments
    /// * `value` - The `FavoritesResponse` to convert.
    ///
    /// # Returns
    /// A `Result` containing a `Vec<PostInternal>` on success, or an `Error` if the API
    /// returned an error or the format was unexpected.
    fn try_from(value: FavoritesResponse) -> Result<Self> {
        match value {
            FavoritesResponse::Succ(FavoritesSucc { favorites, .. }) => {
                debug!("got {} favorites", favorites.len());
                let posts = favorites
                    .into_iter()
                    .map(|post| post.status)
                    .collect::<Vec<PostInternal>>();
                Ok(posts)
            }
            FavoritesResponse::Fail(err) => {
                error!("failed to get favorites: {err:?}");
                Err(Error::ApiError(err))
            }
        }
    }
}

impl From<FavoritesSucc> for Vec<PostInternal> {
    /// Converts a successful favorites response directly into a vector of `PostInternal` objects.
    ///
    /// This conversion is infallible as it assumes a successful `FavoritesSucc` has already been established.
    ///
    /// # Arguments
    /// * `value` - The `FavoritesSucc` struct to convert.
    ///
    /// # Returns
    /// A `Vec<PostInternal>` containing the favorited posts.
    fn from(value: FavoritesSucc) -> Self {
        value.favorites.into_iter().map(|p| p.status).collect()
    }
}

/// Trait for API clients that can interact with Weibo's favorites.
#[async_trait]
pub trait FavoritesApi {
    /// Fetches a page of the user's favorited posts.
    ///
    /// # Arguments
    /// * `page` - The page number to fetch (1-indexed).
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Post>` on success, or an `Error` on failure.
    async fn favorites(&self, page: u32) -> Result<Vec<Post>>;

    /// Destroys (unfavorites) a specific post.
    ///
    /// # Arguments
    /// * `id` - The ID of the post to unfavorite.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    async fn favorites_destroy(&self, id: i64) -> Result<()>;
}

#[async_trait]
impl<C: HttpClient> FavoritesApi for ApiClientImpl<C> {
    /// Fetches a page of the user's favorited posts from the Weibo API.
    ///
    /// The fetched posts are then processed to retrieve any long text or retweeted post details.
    ///
    /// # Arguments
    /// * `page` - The page number of favorites to retrieve.
    ///
    /// # Returns
    /// A `Result` containing a vector of `Post` objects.
    async fn favorites(&self, page: u32) -> Result<Vec<Post>> {
        info!("getting favorites, page: {page}");
        let response = self.client.favorites(page).await?;
        let posts: Vec<PostInternal> = response.json::<FavoritesResponse>().await?.try_into()?;
        let posts = stream::iter(posts)
            .map(|post| self.process_post(post))
            .buffer_unordered(2)
            .collect::<Vec<_>>()
            .await;
        let (oks, _errs): (Vec<_>, Vec<_>) = posts.into_iter().partition_result(); // TODO
        Ok(oks)
    }

    /// Unfavorites a specific post on Weibo.
    ///
    /// # Arguments
    /// * `id` - The ID of the post to unfavorite.
    ///
    /// # Returns
    /// A `Result` indicating success or failure of the unfavorite operation.
    async fn favorites_destroy(&self, id: i64) -> Result<()> {
        info!("destroying favorite, id: {id}");
        self.client.favorites_destroy(id).await?;
        debug!("favorite {id} destroyed");
        Ok(())
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_favorites() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            user: serde_json::Value::Null,
            cookie_store: Default::default(),
        };
        let weibo_api =
            ApiClientImpl::new(SdkApiClient::from_session(mock_client.clone(), session));

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let testcase_path = manifest_dir.join("tests/data/favorites.json");

        mock_client
            .set_favorites_response_from_file(&testcase_path)
            .unwrap();

        weibo_api.favorites(1).await.unwrap();
    }

    #[tokio::test]
    async fn test_favorites_destroy() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            user: serde_json::Value::Null,
            cookie_store: Default::default(),
        };
        let weibo_api =
            ApiClientImpl::new(SdkApiClient::from_session(mock_client.clone(), session));
        let id = 12345;

        mock_client.set_favorites_destroy_response_from_str("{}");

        weibo_api.favorites_destroy(id).await.unwrap();
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    #[tokio::test]
    async fn test_real_favorites() {
        let session_file = "session.json";
        if let Ok(session) = Session::load(session_file) {
            let client = http_client::Client::new().unwrap();
            let weibo_api = ApiClientImpl::new(SdkApiClient::from_session(client, session));
            let _ = weibo_api.favorites(1).await.unwrap();
        }
    }
}
