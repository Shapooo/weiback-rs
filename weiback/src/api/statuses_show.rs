#![allow(async_fn_in_trait)]
use log::{debug, error, info};
use serde::Deserialize;
use weibosdk_rs::http_client::HttpResponse;

use super::{HttpClient, internal::post::PostInternal};
use crate::api::ApiClientImpl;
use crate::models::post::Post;
use crate::{
    error::{Error, Result},
    models::err_response::ErrResponse,
};

#[derive(Debug, Clone, Deserialize)]
pub struct EditConfig {
    #[allow(unused)]
    pub edited: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StatusesShowResponse {
    Succ(PostInternal),
    Fail(ErrResponse),
}

pub trait StatusesShowApi {
    async fn statuses_show(&self, id: i64) -> Result<Post>;
}

impl<C: HttpClient> ApiClientImpl<C> {
    pub(super) async fn statuses_show_internal(&self, id: i64) -> Result<PostInternal> {
        info!("getting long text, id: {id}");

        let response = self.client.statuses_show(id).await?;
        let res = response.json::<StatusesShowResponse>().await?;
        match res {
            StatusesShowResponse::Succ(statuses_show) => {
                debug!("got statuses success");
                Ok(statuses_show)
            }
            StatusesShowResponse::Fail(err) => {
                error!("failed to get long text: {err:?}");
                Err(Error::ApiError(err))
            }
        }
    }
}
impl<C: HttpClient> StatusesShowApi for ApiClientImpl<C> {
    async fn statuses_show(&self, id: i64) -> Result<Post> {
        let ss = self.statuses_show_internal(id).await?;
        self.process_post(ss).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_get_statuses_show() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            screen_name: "test_screen_name".to_string(),
            cookie_store: Default::default(),
        };
        let weibo_api =
            ApiClientImpl::new(SdkApiClient::from_session(mock_client.clone(), session));

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let testcase_path = manifest_dir.join("tests/data/statuses_show.json");
        mock_client
            .set_statuses_show_response_from_file(&testcase_path)
            .unwrap();

        let _post = weibo_api.statuses_show(12345).await.unwrap();
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;
    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    #[tokio::test]
    async fn test_real_get_statuses_show() {
        let session_file = "session.json";
        if let Ok(session) = Session::load(session_file) {
            let client = http_client::Client::new().unwrap();
            let weibo_api = ApiClientImpl::new(SdkApiClient::from_session(client, session));
            let _ = weibo_api.statuses_show(5179586393932632).await.unwrap();
        }
    }
}
