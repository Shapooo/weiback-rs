use std::collections::HashMap;

use weibosdk_rs::{api_client::ApiClient as SdkApiClient, mock::MockClient};

use crate::{
    api::{
        ApiClient, ApiClientImpl, EmojiUpdateApi, FavoritesApi, ProfileStatusesApi, StatusesShowApi,
    },
    error::Result,
    models::post::Post,
};

#[derive(Clone)]
pub struct MockApi {
    client: ApiClientImpl<MockClient>,
}

impl MockApi {
    pub fn new(client: MockClient) -> Self {
        Self {
            client: ApiClientImpl::new(SdkApiClient::from_session(client, Default::default())),
        }
    }
}

impl EmojiUpdateApi for MockApi {
    async fn emoji_update(&self) -> Result<HashMap<String, String>> {
        self.client.emoji_update().await
    }
}

impl FavoritesApi for MockApi {
    async fn favorites(&self, page: u32) -> Result<Vec<Post>> {
        self.client.favorites(page).await
    }

    async fn favorites_destroy(&self, id: i64) -> Result<()> {
        self.client.favorites_destroy(id).await
    }
}

impl StatusesShowApi for MockApi {
    async fn statuses_show(&self, id: i64) -> Result<Post> {
        self.client.statuses_show(id).await
    }
}

impl ProfileStatusesApi for MockApi {
    async fn profile_statuses(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.client.profile_statuses(uid, page).await
    }

    async fn profile_statuses_original(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.client.profile_statuses_original(uid, page).await
    }

    async fn profile_statuses_picture(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.client.profile_statuses_picture(uid, page).await
    }

    async fn profile_statuses_video(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.client.profile_statuses_video(uid, page).await
    }

    async fn profile_statuses_article(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.client.profile_statuses_article(uid, page).await
    }
}

impl ApiClient for MockApi {}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn get_test_data_path(file_name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/")
            .join(file_name)
    }

    fn create_logged_in_api() -> (MockClient, MockApi) {
        let mock_client = MockClient::new();
        let api = MockApi::new(mock_client.clone());
        (mock_client, api)
    }

    #[tokio::test]
    async fn test_emoji_update() {
        let (mock_client, api) = create_logged_in_api();
        mock_client
            .set_emoji_update_response_from_file(&get_test_data_path("emoji.json"))
            .unwrap();
        mock_client
            .set_web_emoticon_response_from_file(&get_test_data_path("web_emoji.json"))
            .unwrap();
        let result = api.emoji_update().await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_favorites() {
        let (mock_client, api) = create_logged_in_api();
        mock_client
            .set_favorites_response_from_file(&get_test_data_path("favorites.json"))
            .unwrap();
        let result = api.favorites(1).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_favorites_destroy() {
        let (mock_client, api) = create_logged_in_api();
        mock_client.set_favorites_destroy_response_from_str("");
        api.favorites_destroy(123).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_statuses_show() {
        let (mock_client, api) = create_logged_in_api();
        mock_client
            .set_statuses_show_response_from_file(&get_test_data_path("statuses_show.json"))
            .unwrap();
        let _ = api.statuses_show(123).await.unwrap();
    }

    #[tokio::test]
    async fn test_profile_statuses() {
        let (mock_client, api) = create_logged_in_api();
        mock_client
            .set_profile_statuses_response_from_file(&get_test_data_path("profile_statuses.json"))
            .unwrap();
        let result = api.profile_statuses(1786055427, 1).await.unwrap();
        assert!(!result.is_empty());
    }
}
