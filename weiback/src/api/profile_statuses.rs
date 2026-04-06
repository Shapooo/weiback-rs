//! This module provides an API for fetching a user's statuses (posts).
//!
//! It includes functionality to retrieve posts from a specific user's timeline,
//! handle various container types, and process the API responses into `Post` models.
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use serde::Deserialize;
use tracing::{debug, error, info};
use weibosdk_rs::http_client::{HttpClient, HttpResponse};

/// Re-exports `weibosdk_rs::profile_statuses::ContainerType` for convenience.
///
/// This enum specifies the type of container to fetch statuses from (e.g., original posts, all posts).
pub use weibosdk_rs::profile_statuses::ContainerType;

use super::{ApiClientImpl, internal::post::PostInternal};
use crate::{
    error::{Error, Result},
    models::Post,
    models::err_response::ErrResponse,
};

/// Represents a single card in the profile statuses API response.
///
/// A card typically contains a microblog (`mblog`) which is a `PostInternal`.
#[derive(Debug, Clone, Deserialize)]
pub struct Card {
    /// The type of the card.
    #[allow(unused)]
    pub card_type: i32,
    /// The microblog (post) contained within this card, if any.
    pub mblog: Option<PostInternal>,
}

/// An enum representing the possible responses from the profile statuses API endpoint,
/// which can either be a successful data payload or an error.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ProfileStatusesResponse {
    Succ(ProfileStatusesSucc),
    Fail(ErrResponse),
}

/// Represents the successful response structure for fetching profile statuses.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileStatusesSucc {
    /// A list of cards, each potentially containing a post.
    pub cards: Vec<Card>,
}

/// Trait for API clients that can fetch a user's profile statuses.
#[async_trait]
pub trait ProfileStatusesApi {
    /// Fetches a page of posts from a specific user's profile.
    ///
    /// # Arguments
    /// * `uid` - The unique ID of the user whose statuses are to be fetched.
    /// * `page` - The page number to fetch (1-indexed).
    /// * `container_type` - The type of container to fetch statuses from (e.g., original posts).
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Post>` on success, or an `Error` on failure.
    async fn profile_statuses(
        &self,
        uid: i64,
        page: u32,
        container_type: ContainerType,
        count: u32,
    ) -> Result<Vec<Post>>;
}

#[async_trait]
impl<C: HttpClient> ProfileStatusesApi for ApiClientImpl<C> {
    /// Fetches a page of posts from a specific user's profile from the Weibo API.
    ///
    /// The fetched posts are then processed to retrieve any long text or retweeted post details,
    /// and filtered to ensure the correct user's posts are returned.
    ///
    /// # Arguments
    /// * `uid` - The unique ID of the user whose statuses are to be fetched.
    /// * `page` - The page number to fetch.
    /// * `container_type` - The type of container to fetch statuses from.
    ///
    /// # Returns
    /// A `Result` containing a vector of `Post` objects.
    async fn profile_statuses(
        &self,
        uid: i64,
        page: u32,
        containter_type: ContainerType,
        count: u32,
    ) -> Result<Vec<Post>> {
        info!(
            "getting profile statuses, uid: {uid}, page: {page}, count: {count}, type: {:?}",
            containter_type
        );
        let response = self
            .client
            .profile_statuses(uid, page, containter_type, count)
            .await
            .inspect_err(|e| {
                error!("profile_statuses(uid={uid}, page={page}) SDK call failed: {e}");
            })?;
        let response = response
            .json::<ProfileStatusesResponse>()
            .await
            .inspect_err(|e| {
                error!("parse ProfileStatusesResponse failed: {e}");
            })?;
        match response {
            ProfileStatusesResponse::Succ(ProfileStatusesSucc { cards }) => {
                let posts_iterator = cards.into_iter().filter_map(|card| card.mblog);

                let posts = posts_iterator
                    .filter(|post| post.user.as_ref().is_none_or(|u| u.id == uid))
                    .collect::<Vec<PostInternal>>();
                let posts = stream::iter(posts)
                    .map(|post| self.process_post(post))
                    .buffer_unordered(2)
                    .collect::<Vec<_>>()
                    .await;
                let (oks, _errs): (Vec<_>, Vec<_>) = posts.into_iter().partition_result(); // TODO
                debug!("got {} posts", oks.len());
                Ok(oks)
            }
            ProfileStatusesResponse::Fail(err) => {
                error!("failed to get profile statuses: {err:?}");
                Err(Error::ApiError(err))
            }
        }
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_profile_statuses_ori() {
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
        let testcase_path = manifest_dir.join("tests/data/profile_statuses.json");
        mock_client
            .set_profile_statuses_response_from_file(&testcase_path)
            .unwrap();

        weibo_api
            .profile_statuses(12345, 1, Default::default(), 20)
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    #[tokio::test]
    async fn test_real_profile_statuses() {
        let session_file = "session.json";
        if let Ok(session) = Session::load(session_file) {
            let client = http_client::Client::new().unwrap();
            let weibo_api = ApiClientImpl::new(SdkApiClient::from_session(client, session));
            let posts = weibo_api
                .profile_statuses(1401527553, 1, Default::default(), 20)
                .await
                .unwrap();
            assert!(!posts.is_empty());
        }
    }
}
