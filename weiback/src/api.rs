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

#[derive(Debug, Clone)]
pub struct ApiClientImpl<C: HttpClient> {
    pub client: SdkApiClient<C>,
}

impl<C: HttpClient> ApiClientImpl<C> {
    pub fn new(client: SdkApiClient<C>) -> Self {
        ApiClientImpl { client }
    }

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

pub type DefaultApiClient = ApiClientImpl<weibosdk_rs::Client>;
#[cfg(feature = "dev-mode")]
pub type DevApiClient = ApiClientImpl<crate::dev_client::DevClient>;
