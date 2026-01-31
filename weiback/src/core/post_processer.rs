use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, info};
use url::Url;

use super::task::TaskContext;
use crate::api::ApiClient;
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::media_downloader::MediaDownloader;
use crate::models::{PicInfoType, Picture, PictureDefinition, PictureMeta, Post, VideoMeta};
use crate::storage::Storage;
use crate::utils::extract_all_pic_metas;

#[derive(Debug, Clone)]
pub struct PostProcesser<A: ApiClient, S: Storage, D: MediaDownloader> {
    storage: S,
    downloader: D,
    emoji_map: EmojiMap<A>,
}

impl<A: ApiClient, S: Storage, D: MediaDownloader> PostProcesser<A, S, D> {
    pub fn new(storage: S, downloader: D, emoji_map: EmojiMap<A>) -> Result<Self> {
        info!("Initializing PostProcesser...");
        info!("PostProcesser initialized successfully.");
        Ok(Self {
            storage,
            downloader,
            emoji_map,
        })
    }

    pub async fn process(&self, ctx: Arc<TaskContext>, posts: Vec<Post>) -> Result<()> {
        info!("Processing {} posts for task {}.", posts.len(), ctx.task_id);
        let pic_quality = ctx.config.picture_definition;
        debug!("Picture definition set to: {pic_quality:?}");

        let emoji_map = self.emoji_map.get_or_try_init().await.ok();

        self.handle_picture(ctx.clone(), &posts, pic_quality, emoji_map)
            .await?;
        self.handle_livephoto_video(ctx.clone(), &posts).await?;

        info!("Finished downloading pictures. Processing posts...");
        stream::iter(posts)
            .map(Ok)
            .try_for_each_concurrent(2, |post| async move {
                if self.need_insert(&post).await? {
                    self.storage.save_post(&post).await
                } else {
                    Ok(())
                }
            })
            .await?;

        info!("Finished processing posts for task {}.", ctx.task_id);
        Ok(())
    }

    async fn need_insert(&self, post: &Post) -> Result<bool> {
        Ok(is_valid_post(post) || self.storage.get_post(post.id).await?.is_none())
    }

    async fn handle_picture(
        &self,
        ctx: Arc<TaskContext>,
        posts: &[Post],
        pic_quality: PictureDefinition,
        emoji_map: Option<&HashMap<String, Url>>,
    ) -> Result<()> {
        let pic_metas = extract_all_pic_metas(posts, pic_quality, emoji_map);
        info!("Found {} unique pictures to download.", pic_metas.len());

        stream::iter(pic_metas)
            .map(Ok)
            .try_for_each_concurrent(10, |meta| {
                let ctx_clone = ctx.clone();
                async move { self.download_pic_to_local(ctx_clone, meta).await }
            })
            .await?;
        Ok(())
    }

    async fn download_pic_to_local(
        &self,
        ctx: Arc<TaskContext>,
        pic_meta: PictureMeta,
    ) -> Result<()> {
        let url = pic_meta.url().to_owned();
        if self.storage.picture_saved(ctx.clone(), &url).await? {
            debug!("Picture {url} already exists in local storage, skipping download.");
            return Ok(());
        }
        debug!("Downloading picture {url} to local storage.");
        let storage = self.storage.clone();
        let callback = Box::new(
            move |ctx, blob| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async move {
                    let pic = Picture {
                        meta: pic_meta,
                        blob,
                    };
                    storage.save_picture(ctx, &pic).await?;
                    Ok(())
                })
            },
        );

        self.downloader.download_media(ctx, &url, callback).await?;
        Ok(())
    }

    async fn handle_livephoto_video(&self, ctx: Arc<TaskContext>, posts: &[Post]) -> Result<()> {
        let video_metas = extract_livephoto_video_metas(posts);
        info!(
            "Found {} unique livephoto videos to download.",
            video_metas.len()
        );

        stream::iter(video_metas)
            .map(Ok)
            .try_for_each_concurrent(10, |meta| {
                let ctx_clone = ctx.clone();
                async move { self.download_video_to_local(ctx_clone, meta).await }
            })
            .await?;
        Ok(())
    }

    async fn download_video_to_local(
        &self,
        ctx: Arc<TaskContext>,
        video_meta: VideoMeta,
    ) -> Result<()> {
        let url = video_meta.url().to_owned();
        if self.storage.video_saved(ctx.clone(), &url).await? {
            debug!("Video {url} already exists in local storage, skipping download.");
            return Ok(());
        }
        debug!("Downloading video {url} to local storage.");
        let storage = self.storage.clone();
        let callback = Box::new(
            move |ctx, blob| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async move {
                    let video = crate::models::Video {
                        meta: video_meta,
                        blob,
                    };
                    storage.save_video(ctx, &video).await?;
                    Ok(())
                })
            },
        );

        self.downloader.download_media(ctx, &url, callback).await?;
        Ok(())
    }
}

fn is_valid_post(post: &Post) -> bool {
    post.user.is_some()
        && (post.retweeted_status.is_none()
            || post.retweeted_status.as_ref().unwrap().user.is_some())
}

fn extract_livephoto_video_metas(posts: &[Post]) -> Vec<VideoMeta> {
    let mut metas = Vec::new();
    let mut seen_urls = HashSet::new();

    for post in posts.iter().flat_map(post_and_retweeted) {
        if let Some(pic_infos) = &post.pic_infos {
            for pic_info in pic_infos.values() {
                if let PicInfoType::Livephoto = pic_info.r#type
                    && let Some(video_url) = &pic_info.video
                    && seen_urls.insert(video_url.clone())
                {
                    metas.push(VideoMeta {
                        url: video_url.clone(),
                        post_id: post.id,
                    });
                }
            }
        }
    }
    metas
}

fn post_and_retweeted(post: &Post) -> impl Iterator<Item = &Post> {
    std::iter::once(post).chain(post.retweeted_status.as_deref())
}
