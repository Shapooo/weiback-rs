use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, error, info};
use tokio::sync::mpsc;

use crate::api::ApiClient;
use crate::config::get_config;
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::media_downloader::MediaDownloader;
use crate::message::{ErrMsg, ErrType, Message};
use crate::models::{Picture, PictureDefinition, PictureMeta, Post};
use crate::storage::Storage;
use crate::utils::extract_all_pic_metas;

#[derive(Debug, Clone)]
pub struct PostProcesser<A: ApiClient, S: Storage, D: MediaDownloader> {
    api_client: A,
    storage: S,
    downloader: D,
    emoji_map: EmojiMap<A>,
    msg_sender: mpsc::Sender<Message>,
}

impl<A: ApiClient, S: Storage, D: MediaDownloader> PostProcesser<A, S, D> {
    pub fn new(
        api_client: A,
        storage: S,
        downloader: D,
        emoji_map: EmojiMap<A>,
        msg_sender: mpsc::Sender<Message>,
    ) -> Result<Self> {
        info!("Initializing PostProcesser...");
        info!("PostProcesser initialized successfully.");
        Ok(Self {
            api_client,
            storage,
            downloader,
            emoji_map,
            msg_sender,
        })
    }

    pub async fn process(&self, task_id: u64, posts: Vec<Post>) -> Result<()> {
        info!("Processing {} posts for task {}.", posts.len(), task_id);
        let pic_quality = get_config().read()?.picture_definition;
        debug!("Picture definition set to: {pic_quality:?}");

        let emoji_map = self.emoji_map.get_or_try_init().await.ok();

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

    async fn handle_long_text(&self, post: &mut Post, task_id: u64) -> Result<()> {
        if post.is_long_text {
            debug!("Fetching long text for post {}.", post.id);
            match self.api_client.statuses_show(post.id).await {
                Ok(n_post) => {
                    post.text = n_post.long_text.unwrap();
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
