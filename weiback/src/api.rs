pub mod emoji;
pub mod favorites;
pub(crate) mod internal;
pub mod profile_statuses;
pub mod statuses_show;

use log::warn;
use weibosdk_rs::{ApiClient as SdkApiClient, http_client::HttpClient};

use crate::error::Result;
use crate::models::Post;
use internal::{
    post::PostInternal,
    url_struct::{UrlStructInternal, UrlTypeInternal},
};

pub use emoji::EmojiUpdateApi;
pub use favorites::FavoritesApi;
pub use profile_statuses::ProfileStatusesApi;
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

    pub async fn process_post(&self, mut post: PostInternal) -> Result<Post> {
        self.handle_long_text(&mut post).await?;

        if post.url_struct.is_some() && post.retweeted_status.is_some() {
            self.refine_url_struct(&mut post).await?;
        }
        post.try_into()
    }

    pub async fn refine_url_struct(&self, post: &mut PostInternal) -> Result<()> {
        let text = post.text.as_str();
        let (outter, inner): (Vec<_>, Vec<_>) = post
            .url_struct
            .take()
            .unwrap()
            .0
            .into_iter()
            .partition(|u| match u.url_type {
                UrlTypeInternal::Picture | UrlTypeInternal::Link => text.contains(&u.short_url),
                UrlTypeInternal::Location => true,
                UrlTypeInternal::Appendix => false,
                UrlTypeInternal::Topic => false,
            });
        post.url_struct = (!outter.is_empty()).then_some(UrlStructInternal(outter));
        if let Some(ret) = post.retweeted_status.as_mut()
            && ret.url_struct.is_none()
            && !inner.is_empty()
        {
            ret.url_struct = Some(UrlStructInternal(inner));
        }
        Ok(())
    }

    pub async fn handle_long_text(&self, post: &mut PostInternal) -> Result<()> {
        if post.is_long_text {
            *post = self.statuses_show_internal(post.id).await?;
            if let Some(long_text) = post.long_text.take() {
                post.text = long_text.content; // should be Some
            } else {
                let id = post.id;
                warn!("post {id} is_long_text without long_text");
            }
        }
        if let Some(ret) = post.retweeted_status.as_mut()
            && ret.is_long_text
        {
            **ret = self.statuses_show_internal(ret.id).await?;
            if let Some(long_text) = ret.long_text.take() {
                ret.text = long_text.content; // should be Some
            } else {
                let id = post.id;
                warn!("post {id} is_long_text without long_text");
            }
        }
        Ok(())
    }
}

impl<C: HttpClient> ApiClient for ApiClientImpl<C> {}

pub type DefaultApiClient = ApiClientImpl<weibosdk_rs::Client>;
#[cfg(feature = "dev-mode")]
pub type DevApiClient = ApiClientImpl<crate::dev_client::DevClient>;
