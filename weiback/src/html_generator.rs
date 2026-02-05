pub mod view_model;

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use lazy_static::lazy_static;
use log::{debug, info};
use tera::{Context, Tera};
use url::Url;

use crate::api::EmojiUpdateApi;
use crate::core::task::TaskContext;
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::exporter::{HTMLPage, PictureExport};
use crate::models::{PictureDefinition, PictureMeta, Post};
use crate::storage::Storage;
use crate::utils::{extract_all_pic_metas, make_resource_dir_name, url_to_filename};
use view_model::PostView;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_template("page.html", include_str!("../templates/page.html"))
            .unwrap();
        tera.add_raw_template("post.html", include_str!("../templates/post.html"))
            .unwrap();
        tera.add_raw_template("posts.html", include_str!("../templates/posts.html"))
            .unwrap();
        tera.autoescape_on(Vec::new());
        tera
    };
}

#[derive(Debug, Clone)]
pub struct HTMLGenerator<E: EmojiUpdateApi, S: Storage> {
    storage: S,
    emoji_map: EmojiMap<E>,
}

impl<E: EmojiUpdateApi, S: Storage> HTMLGenerator<E, S> {
    pub fn new(emoji_map: EmojiMap<E>, storage: S) -> Self {
        Self { storage, emoji_map }
    }

    fn generate_post(
        &self,
        post: Post,
        page_name: &str,
        emoji_map: Option<&HashMap<String, Url>>,
        definition: PictureDefinition,
    ) -> Result<String> {
        let pic_folder = make_resource_dir_name(page_name);
        let post_view = PostView::from_post(post, &pic_folder, definition, emoji_map)?;

        let context = Context::from_serialize(post_view)?;
        let html = TEMPLATES.render("post.html", &context)?;
        Ok(html)
    }

    async fn generate_page(
        &self,
        posts: Vec<Post>,
        page_name: &str,
        pic_quality: PictureDefinition,
    ) -> Result<String> {
        let emoji_map = self.emoji_map.get_or_try_init().await.ok();
        info!("Generating page for {} posts", posts.len());
        let posts_html = posts
            .into_iter()
            .map(|p| self.generate_post(p, page_name, emoji_map, pic_quality))
            .collect::<Result<Vec<_>>>()?;
        let posts_html = posts_html.join("");
        let mut context = Context::new();
        context.insert("html", &posts_html);
        let html = TEMPLATES.render("page.html", &context)?;
        info!("Successfully generated page");
        Ok(html)
    }

    pub async fn generate_html(
        &self,
        ctx: Arc<TaskContext>,
        posts: Vec<Post>,
        page_name: &str,
    ) -> Result<HTMLPage> {
        info!("Generating HTML for {} posts.", posts.len());
        let pic_quality = ctx.config.picture_definition;
        let emoji_map = self.emoji_map.get_or_try_init().await.ok();
        debug!("Using picture quality: {pic_quality:?}");
        let pic_metas = extract_all_pic_metas(&posts, pic_quality, emoji_map);
        info!(
            "Found {} unique pictures for HTML generation.",
            pic_metas.len()
        );
        let pic_futures = pic_metas
            .into_iter()
            .map(|m| self.get_picture_export_info(m));
        let pictures_to_export: Vec<PictureExport> = stream::iter(pic_futures)
            .buffer_unordered(8)
            .filter_map(|result| async move {
                match result {
                    Ok(Some(info)) => Some(info),
                    Ok(None) => None,
                    Err(e) => {
                        log::warn!("Failed to get picture export info: {}", e);
                        None
                    }
                }
            })
            .collect()
            .await;
        debug!(
            "Found {} pictures to export from local storage.",
            pictures_to_export.len()
        );
        let content = self.generate_page(posts, page_name, pic_quality).await?;
        info!("HTML content generated successfully.");
        Ok(HTMLPage {
            html: content,
            pictures_to_export,
        })
    }

    async fn get_picture_export_info(
        &self,
        pic_meta: PictureMeta,
    ) -> Result<Option<PictureExport>> {
        let url = pic_meta.url();
        if let Some(source_path) = self.storage.get_picture_path(url).await? {
            let target_file_name = url_to_filename(url)?;
            Ok(Some(PictureExport {
                source_path,
                target_file_name,
            }))
        } else {
            log::warn!("Picture path not found in storage for url: {}", url);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use weibosdk_rs::mock::MockClient;

    use super::*;
    use crate::{
        api::{FavoritesApi, ProfileStatusesApi},
        mock::MockApi,
        storage::{StorageImpl, database},
    };

    async fn create_test_storage() -> StorageImpl {
        let db_pool = database::create_db_pool_with_url(":memory:").await.unwrap();
        StorageImpl::new(db_pool)
    }

    fn create_mock_client() -> MockClient {
        MockClient::new()
    }

    fn create_mock_api(client: &MockClient) -> MockApi {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        client
            .set_favorites_response_from_file(
                manifest_dir.join("tests/data/favorites.json").as_path(),
            )
            .unwrap();
        client
            .set_profile_statuses_response_from_file(
                manifest_dir
                    .join("tests/data/profile_statuses.json")
                    .as_path(),
            )
            .unwrap();
        client
            .set_emoji_update_response_from_file(
                manifest_dir.join("tests/data/emoji.json").as_path(),
            )
            .unwrap();
        client
            .set_web_emoticon_response_from_file(
                manifest_dir.join("tests/data/web_emoji.json").as_path(),
            )
            .unwrap();
        MockApi::new(client.clone())
    }

    async fn create_generator(api: &MockApi) -> HTMLGenerator<MockApi, StorageImpl> {
        let storage = create_test_storage().await;
        let emoji_map = EmojiMap::new(api.clone());
        HTMLGenerator::new(emoji_map, storage)
    }

    async fn create_posts(api: &MockApi) -> Vec<Post> {
        let mut posts = api.favorites(0).await.unwrap();
        posts.extend(api.profile_statuses(1786055427, 0).await.unwrap());
        posts
    }

    #[tokio::test]
    async fn test_generate_post_with_valid_emoji() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let generator = create_generator(&api).await;
        let emoji_map = api.emoji_update().await.unwrap();
        let definition = PictureDefinition::Original;

        for post in posts {
            generator
                .generate_post(post, "test", Some(&emoji_map), definition)
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_generate_post_with_invalid_emoji() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let definition = PictureDefinition::Original;
        let generator = create_generator(&api).await;
        for post in posts {
            generator
                .generate_post(post, "test", None, definition)
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_generate_page() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let generator = create_generator(&api).await;
        let definition = PictureDefinition::Original;
        generator
            .generate_page(posts, "test_page", definition)
            .await
            .unwrap();
    }
}
