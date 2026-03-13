//! This module handles the post-processing phase of a task.
//!
//! It is responsible for:
//! 1.  Extracting media metadata (images, videos, emojis, avatars) from posts.
//! 2.  Downloading media files to local storage using a [`MediaDownloader`].
//! 3.  Enriching post data (e.g., mapping emojis to local IDs).
//! 4.  Saving processed posts into the [`Storage`].

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use tracing::{debug, info};
use url::Url;

use super::task::TaskContext;
use crate::api::ApiClient;
use crate::core::task::PostInfo;
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::media_downloader::MediaDownloader;
use crate::models::{PicInfoType, Picture, PictureDefinition, PictureMeta, Post, VideoMeta};
use crate::storage::Storage;
use crate::utils::{
    extract_all_pic_metas, extract_emojis_from_text, extract_inline_pic_ids,
    extract_standalone_pic_ids, pic_url_to_id,
};

/// A processor that handles media downloading and post data enrichment.
///
/// `PostProcesser` implements the logic for extracting and downloading all media
/// associated with a set of posts, ensuring they are stored locally before the
/// posts themselves are saved to the database.
#[derive(Debug, Clone)]
pub struct PostProcesser<A: ApiClient, S: Storage, D: MediaDownloader> {
    storage: S,
    downloader: D,
    emoji_map: EmojiMap<A>,
}

impl<A: ApiClient, S: Storage, D: MediaDownloader> PostProcesser<A, S, D> {
    /// Creates a new `PostProcesser` instance.
    pub fn new(storage: S, downloader: D, emoji_map: EmojiMap<A>) -> Result<Self> {
        info!("Initializing PostProcesser...");
        info!("PostProcesser initialized successfully.");
        Ok(Self {
            storage,
            downloader,
            emoji_map,
        })
    }

    /// Checks if any image associated with a post (standalone or inline) is missing locally.
    pub async fn is_any_image_missing(&self, ctx: Arc<TaskContext>, post: &Post) -> Result<bool> {
        let mut all_ids = extract_standalone_pic_ids(post);
        all_ids.extend(extract_inline_pic_ids(post));

        for id in all_ids {
            let infos = self.storage.get_pictures_by_id(&id).await?;
            if infos.is_empty() {
                return Ok(true);
            }
            // Check if at least one version of this picture is saved on disk
            let mut found = false;
            for info in infos {
                if self
                    .storage
                    .picture_saved(ctx.clone(), info.meta.url())
                    .await?
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Enriches a raw `Post` with additional information for UI display.
    ///
    /// This includes mapping avatars, emojis, and pictures to their local identifiers.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `post` - The post to enrich.
    pub async fn build_post_info(&self, post: Post) -> Result<PostInfo> {
        let avatar_id = if let Some(user) = &post.user {
            self.storage
                .get_avatar_info(user.id)
                .await?
                .map(|info| pic_url_to_id(info.meta.url()))
                .transpose()?
        } else {
            None
        };

        let standalone_ids = extract_standalone_pic_ids(&post);

        let mut inline_map = HashMap::new();
        if let Some(url_struct) = &post.url_struct {
            for item in url_struct.0.iter() {
                if let Some(pic_info) = item.pic_infos.as_ref() {
                    inline_map.insert(item.short_url.clone(), pic_info.pic_id.clone());
                }
            }
        }

        let mut emoji_map = HashMap::new();
        if let Ok(all_emoji_map) = self.emoji_map.get_or_try_init().await {
            for emoji_text in extract_emojis_from_text(&post.text) {
                if let Some(url) = all_emoji_map.get(emoji_text)
                    && let Ok(id) = pic_url_to_id(url)
                {
                    emoji_map.insert(emoji_text.to_owned(), id);
                }
            }
            if let Some(retweeted) = &post.retweeted_status {
                for emoji_text in extract_emojis_from_text(&retweeted.text) {
                    if let Some(url) = all_emoji_map.get(emoji_text)
                        && let Ok(id) = pic_url_to_id(url)
                    {
                        emoji_map.insert(emoji_text.to_owned(), id);
                    }
                }
            }
        }

        Ok(PostInfo {
            post,
            avatar_id,
            emoji_map,
            standalone_ids,
            inline_map,
        })
    }

    /// Processes a batch of posts, downloading media and saving them to storage.
    ///
    /// This is the main entry point for persisting posts fetched from the API.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `posts` - The list of posts to process.
    #[tracing::instrument(skip(self, ctx, posts), fields(task_id = ctx.task_id, batch_size = posts.len()))]
    pub async fn process(&self, ctx: Arc<TaskContext>, posts: Vec<Post>) -> Result<()> {
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

        info!("Finished processing posts for task {:?}.", ctx.task_id);
        Ok(())
    }

    /// Determines if a post needs to be inserted or updated in storage.
    async fn need_insert(&self, post: &Post) -> Result<bool> {
        Ok(is_valid_post(post) || self.storage.get_post(post.id).await?.is_none())
    }

    /// Identifies and downloads all unique pictures found in a batch of posts.
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

    /// Downloads a single picture and saves it to local storage.
    #[tracing::instrument(skip(self, ctx, pic_meta), fields(url = %pic_meta.url()))]
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

    /// Identifies and downloads all unique LivePhoto videos found in a batch of posts.
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

    /// Downloads a single video and saves it to local storage.
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
