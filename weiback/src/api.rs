pub mod emoji;
pub mod favorites;
pub mod profile_statuses;
pub mod statuses_show;

use weibosdk_rs::{ApiClient as SdkApiClient, http_client::HttpClient};

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
}

impl<C: HttpClient> ApiClient for ApiClientImpl<C> {}

pub type DefaultApiClient = ApiClientImpl<weibosdk_rs::Client>;
