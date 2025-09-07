#![allow(async_fn_in_trait)]
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use log::{debug, error, info};
use serde::Deserialize;
use weibosdk_rs::http_client::{HttpClient, HttpResponse};

use super::ApiClientImpl;
use super::internal::post::PostInternal;
use crate::{
    error::{Error, Result},
    models::{Post, err_response::ErrResponse},
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FavoritesPost {
    pub status: PostInternal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum FavoritesResponse {
    Succ(FavoritesSucc),
    Fail(ErrResponse),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FavoritesSucc {
    pub favorites: Vec<FavoritesPost>,
    #[serde(default)]
    #[allow(unused)]
    pub total_number: i32,
}

impl TryFrom<FavoritesResponse> for Vec<PostInternal> {
    type Error = Error;
    fn try_from(value: FavoritesResponse) -> Result<Self> {
        let res = value;
        match res {
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
    fn from(value: FavoritesSucc) -> Self {
        value.favorites.into_iter().map(|p| p.status).collect()
    }
}

pub trait FavoritesApi {
    async fn favorites(&self, page: u32) -> Result<Vec<Post>>;
    async fn favorites_destroy(&self, id: i64) -> Result<()>;
}

impl<C: HttpClient> FavoritesApi for ApiClientImpl<C> {
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
    use std::sync::{Arc, Mutex};

    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_favorites() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            screen_name: "test_screen_name".to_string(),
            cookie_store: Default::default(),
        };
        let session = Arc::new(Mutex::new(session));
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
            screen_name: "test_screen_name".to_string(),
            cookie_store: Default::default(),
        };
        let session = Arc::new(Mutex::new(session));
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
    use std::sync::{Arc, Mutex};
    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    #[tokio::test]
    async fn test_real_favorites() {
        let session_file = "session.json";
        if let Ok(session) = Session::load(session_file) {
            let session = Arc::new(Mutex::new(session));
            let client = http_client::Client::new().unwrap();
            let weibo_api = ApiClientImpl::new(SdkApiClient::from_session(client, session));
            let _ = weibo_api.favorites(1).await.unwrap();
        }
    }
}
