#![allow(async_fn_in_trait)]
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use log::{debug, error, info};
use serde::Deserialize;
use weibosdk_rs::{
    http_client::{HttpClient, HttpResponse},
    profile_statuses::ContainerType,
};

use super::{ApiClientImpl, internal::post::PostInternal};
use crate::{
    error::{Error, Result},
    models::Post,
    models::err_response::ErrResponse,
};

#[derive(Debug, Clone, Deserialize)]
struct Card {
    #[allow(unused)]
    card_type: i32,
    mblog: Option<PostInternal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ProfileStatusesResponse {
    Succ(ProfileStatusesSucc),
    Fail(ErrResponse),
}

#[derive(Debug, Clone, Deserialize)]
struct ProfileStatusesSucc {
    cards: Vec<Card>,
}

pub trait ProfileStatusesApi {
    async fn profile_statuses(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
    async fn profile_statuses_original(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
    async fn profile_statuses_picture(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
    async fn profile_statuses_video(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
    async fn profile_statuses_article(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
}

impl<C: HttpClient> ApiClientImpl<C> {
    async fn do_profile_statuses(
        &self,
        uid: i64,
        page: u32,
        r#type: ContainerType,
    ) -> Result<Vec<Post>> {
        info!(
            "getting profile statuses, uid: {uid}, page: {page}, type: {:?}",
            r#type
        );
        let response = self.client.profile_statuses(uid, page, r#type).await?;
        let response = response.json::<ProfileStatusesResponse>().await?;
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

impl<C: HttpClient> ProfileStatusesApi for ApiClientImpl<C> {
    async fn profile_statuses(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let r#type = ContainerType::Normal;
        self.do_profile_statuses(uid, page, r#type).await
    }

    async fn profile_statuses_original(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let r#type = ContainerType::Original;
        self.do_profile_statuses(uid, page, r#type).await
    }

    async fn profile_statuses_picture(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let r#type = ContainerType::Picture;
        self.do_profile_statuses(uid, page, r#type).await
    }

    async fn profile_statuses_video(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let r#type = ContainerType::Video;
        self.do_profile_statuses(uid, page, r#type).await
    }

    async fn profile_statuses_article(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let r#type = ContainerType::Article;
        self.do_profile_statuses(uid, page, r#type).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_profile_statuses_ori() {
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
        let testcase_path = manifest_dir.join("tests/data/profile_statuses.json");
        mock_client
            .set_profile_statuses_response_from_file(&testcase_path)
            .unwrap();

        weibo_api.profile_statuses_original(12345, 1).await.unwrap();
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    #[tokio::test]
    async fn test_real_profile_statuses() {
        let session_file = "session.json";
        if let Ok(session) = Session::load(session_file) {
            let client = http_client::Client::new().unwrap();
            let weibo_api = ApiClientImpl::new(SdkApiClient::from_session(
                client,
                Arc::new(Mutex::new(session)),
            ));
            let posts = weibo_api.profile_statuses(1401527553, 1).await.unwrap();
            assert!(!posts.is_empty());
        }
    }
}
