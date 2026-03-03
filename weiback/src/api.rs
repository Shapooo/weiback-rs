//! This module defines the main API client for interacting with the Weibo SDK.
//!
//! It provides a unified trait `ApiClient` that aggregates functionality from various
//! sub-modules (emoji, favorites, profile statuses, statuses show).
//! The primary implementation is `ApiClientImpl`, which wraps the `weibosdk_rs::ApiClient`.

pub mod emoji;
pub mod favorites;
pub(crate) mod internal;
pub mod profile_statuses;
pub mod statuses_show;

use log::warn;
use weibosdk_rs::{ApiClient as SdkApiClient, http_client::HttpClient};

use crate::error::Result;
use crate::models::Post;
use internal::post::PostInternal;

pub use emoji::EmojiUpdateApi;
pub use favorites::FavoritesApi;
pub use profile_statuses::{ContainerType, ProfileStatusesApi};
pub use statuses_show::StatusesShowApi;

/// A trait that combines various Weibo API functionalities.
///
/// Implementors of this trait can perform operations related to emoji updates,
/// managing favorites, fetching profile statuses, and retrieving detailed status information.
pub trait ApiClient:
    emoji::EmojiUpdateApi
    + favorites::FavoritesApi
    + statuses_show::StatusesShowApi
    + profile_statuses::ProfileStatusesApi
    + Send
    + Sync
    + Clone
{
}

/// The default implementation of the `ApiClient` trait.
///
/// It wraps a `weibosdk_rs::ApiClient` instance to provide concrete API call functionality.
#[derive(Debug, Clone)]
pub struct ApiClientImpl<C: HttpClient> {
    pub client: SdkApiClient<C>,
}

impl<C: HttpClient> ApiClientImpl<C> {
    /// Creates a new `ApiClientImpl` instance.
    ///
    /// # Arguments
    /// * `client` - An instance of `weibosdk_rs::ApiClient` that handles the underlying HTTP requests.
    pub fn new(client: SdkApiClient<C>) -> Self {
        ApiClientImpl { client }
    }

    /// Processes a `PostInternal` object, fetching full long text and retweet information if available.
    ///
    /// This method is responsible for hydrating a `PostInternal` from the database with
    /// potentially missing details (like full long text content or complete retweet objects)
    /// by making additional API calls if `is_long_text` is true or `retweeted_status` exists.
    ///
    /// # Arguments
    /// * `post` - The `PostInternal` object to process.
    ///
    /// # Returns
    /// A `Result` containing a fully hydrated `Post` object.
    async fn process_post(&self, mut post: PostInternal) -> Result<Post> {
        // If outer post is long text, fetch its full version.
        if post.is_long_text {
            if post.long_text.is_none() {
                post = self.statuses_show_internal(post.id).await?;
            }
            if let Some(long_text) = post.long_text.take() {
                post.text = long_text.content;
            } else {
                // some is_long_text flag of short post without long_text is wrongly set true by weibo
                // workaround, and warn it
                let id = post.id;
                warn!("post {id} is_long_text without long_text");
            }
        }

        // If there's a retweet, fetch its full version.
        // This also handles the long text of the retweet.
        if let Some(mut retweet_box) = post.retweeted_status.take() {
            let full_retweet = self.statuses_show_internal(retweet_box.id).await?;
            *retweet_box = full_retweet;

            if let Some(long_text) = retweet_box.long_text.take() {
                retweet_box.text = long_text.content;
            } else if retweet_box.is_long_text {
                // workaround wrongly set is_long_text
                let id = retweet_box.id;
                warn!("retweeted post {id} is_long_text without long_text");
            }
            post.retweeted_status = Some(retweet_box);
        }

        post.try_into()
    }
}

impl<C: HttpClient> ApiClient for ApiClientImpl<C> {}

/// A type alias for `ApiClientImpl` using the default `weibosdk_rs::Client`.
pub type DefaultApiClient = ApiClientImpl<weibosdk_rs::Client>;
/// A type alias for `ApiClientImpl` using `crate::dev_client::DevClient` for development mode.
#[cfg(feature = "dev-mode")]
pub type DevApiClient = ApiClientImpl<crate::dev_client::DevClient>;
