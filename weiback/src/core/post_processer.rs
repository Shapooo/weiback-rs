pub mod html_generator;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, error, info};
use tokio::sync::mpsc;
use weibosdk_rs::WeiboAPI;

use crate::config::get_config;
use crate::error::Result;
use crate::exporter::HTMLPage;
use crate::media_downloader::MediaDownloader;
use crate::message::{ErrMsg, ErrType, Message};
use crate::models::{Picture, PictureDefinition, PictureMeta, Post};
use crate::storage::Storage;
use crate::utils::{extract_all_pic_metas, pic_id_to_url, process_in_post_pics};
use html_generator::{HTMLGenerator, create_tera};

#[derive(Debug, Clone)]
pub struct PostProcesser<W: WeiboAPI, S: Storage, D: MediaDownloader> {
    api_client: W,
    storage: S,
    downloader: D,
    html_generator: HTMLGenerator<W, S, D>,
    msg_sender: mpsc::Sender<Message>,
}

impl<W: WeiboAPI, S: Storage, D: MediaDownloader> PostProcesser<W, S, D> {
    pub fn new(
        api_client: W,
        storage: S,
        downloader: D,
        msg_sender: mpsc::Sender<Message>,
    ) -> Result<Self> {
        info!("Initializing PostProcesser...");
        let path = std::env::current_exe()?;
        let tera_path = path
            .parent()
            .expect("the executable should have parent, maybe bugs in there")
            .join("templates");
        debug!("Loading templates from: {tera_path:?}");
        let tera = create_tera(&tera_path)?;
        let html_generator = HTMLGenerator::new(
            api_client.clone(),
            storage.clone(),
            downloader.clone(),
            tera,
        );
        info!("PostProcesser initialized successfully.");
        Ok(Self {
            api_client,
            storage,
            downloader,
            html_generator,
            msg_sender,
        })
    }

    pub async fn process(&self, task_id: u64, posts: Vec<Post>) -> Result<()> {
        info!("Processing {} posts for task {}.", posts.len(), task_id);
        let pic_quality = get_config().read()?.picture_definition;
        debug!("Picture definition set to: {pic_quality:?}");

        let emoji_map = self.html_generator.get_or_try_init_emoji().await.ok();

        self.handle_picture(&posts, pic_quality, emoji_map, task_id)
            .await?;

        info!("Finished downloading pictures. Processing posts...");
        for mut post in posts {
            self.handle_long_text(&mut post, task_id).await?;
            self.storage.save_post(&post).await?;
        }

        info!("Finished processing posts for task {task_id}.");
        Ok(())
    }

    pub async fn generate_html(&self, posts: Vec<Post>, page_name: &str) -> Result<HTMLPage> {
        self.html_generator.generate_html(posts, page_name).await
    }

    async fn handle_long_text(&self, post: &mut Post, task_id: u64) -> Result<()> {
        if post.is_long_text {
            debug!("Fetching long text for post {}.", post.id);
            match self.api_client.get_long_text(post.id).await {
                Ok(long_text) => {
                    post.text = long_text;
                }
                Err(e) => {
                    error!("Failed to fetch long text for post {}: {}", post.id, e);
                    self.msg_sender
                        .send(Message::Err(ErrMsg {
                            r#type: ErrType::LongTextFail { post_id: post.id },
                            task_id,
                            err: e.to_string(),
                        }))
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_picture(
        &self,
        posts: &[Post],
        pic_quality: PictureDefinition,
        emoji_map: Option<&HashMap<String, String>>,
        task_id: u64,
    ) -> Result<()> {
        let pic_metas = extract_all_pic_metas(posts, pic_quality, emoji_map);
        info!("Found {} unique pictures to download.", pic_metas.len());

        stream::iter(pic_metas)
            .map(Ok)
            .try_for_each_concurrent(10, |meta| async move {
                self.download_pic_to_local(task_id, meta).await
            })
            .await?;
        Ok(())
    }

    async fn download_pic_to_local(&self, task_id: u64, pic_meta: PictureMeta) -> Result<()> {
        let url = pic_meta.url().to_string();
        // TODO: add method check existance of picture
        if self.storage.get_picture_blob(&url).await?.is_some() {
            debug!("Picture {url} already exists in local storage, skipping download.");
            return Ok(());
        }
        debug!("Downloading picture {url} to local storage.");
        let storage = self.storage.clone();
        let callback = Box::new(
            move |blob| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async move {
                    let pic = Picture {
                        meta: pic_meta,
                        blob,
                    };
                    storage.save_picture(&pic).await?;
                    Ok(())
                })
            },
        );

        self.downloader
            .download_picture(task_id, url, callback)
            .await?;
        Ok(())
    }
}

pub fn extract_in_post_pic_metas(post: &Post, definition: PictureDefinition) -> Vec<PictureMeta> {
    process_in_post_pics(post, |id, pic_infos, post| {
        pic_id_to_url(id, pic_infos, &definition)
            .map(|url| PictureMeta::in_post(url.to_string(), post.id))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use weibosdk_rs::{
        favorites::FavoritesAPI, mock_api::MockAPI, mock_client::MockClient,
        profile_statuses::ProfileStatusesAPI,
    };

    use super::*;
    use crate::mock::{media_downloader::MediaDownloaderMock, storage::StorageMock};

    fn create_mock_client() -> MockClient {
        MockClient::new()
    }

    fn create_mock_api(client: &MockClient) -> MockAPI {
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
            .set_long_text_response_from_file(
                manifest_dir.join("tests/data/long_text.json").as_path(),
            )
            .unwrap();
        MockAPI::from_session(client.clone(), Default::default())
    }

    async fn create_posts(api: &MockAPI) -> Vec<Post> {
        let mut posts = api.favorites(0).await.unwrap();
        posts.extend(api.profile_statuses(123, 0).await.unwrap());
        posts
    }

    async fn create_processor(
        api: MockAPI,
        msg_sender: mpsc::Sender<Message>,
    ) -> PostProcesser<MockAPI, StorageMock, MediaDownloaderMock> {
        let storage = StorageMock::new();
        let downloader = MediaDownloaderMock::new();
        PostProcesser::new(api, storage, downloader, msg_sender).unwrap()
    }

    #[tokio::test]
    async fn test_extract_all_pic_metas() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let (msg_sender, _) = mpsc::channel(100);
        let processor = create_processor(api.clone(), msg_sender).await;

        let emoji_map = processor
            .html_generator
            .get_or_try_init_emoji()
            .await
            .unwrap();

        let metas = extract_all_pic_metas(&posts, PictureDefinition::Large, Some(emoji_map));

        assert!(
            !metas.is_empty(),
            "No picture metadata was extracted, check test data files."
        );

        let has_in_post = metas
            .iter()
            .any(|m| matches!(m, PictureMeta::InPost { .. }));
        let has_avatar = metas
            .iter()
            .any(|m| matches!(m, PictureMeta::Avatar { .. }));
        let has_emoji = metas.iter().any(|m| m.url().contains("face.t.sinajs.cn"));

        assert!(has_in_post, "Should extract in-post pictures");
        assert!(has_avatar, "Should extract user avatars");
        assert!(has_emoji, "Should extract emoji pictures");
    }
}
