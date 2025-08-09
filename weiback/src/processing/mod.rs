pub mod html_generator;

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use futures::future::try_join_all;
use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, error, info};
use serde_json::Value;
use tokio::sync::mpsc;
use weibosdk_rs::WeiboAPI;

use crate::config::get_config;
use crate::error::{Error, Result};
use crate::exporter::{ExportOptions, HTMLPage, HTMLPicture};
use crate::media_downloader::MediaDownloader;
use crate::message::{ErrMsg, ErrType, Message};
use crate::models::{Picture, PictureDefinition, PictureMeta, Post};
use crate::storage::Storage;
use crate::utils::EMOJI_EXPR;
use html_generator::{HTMLGenerator, create_tera};

#[derive(Debug, Clone)]
pub struct PostProcesser<W: WeiboAPI, S: Storage, D: MediaDownloader> {
    api_client: W,
    storage: S,
    downloader: D,
    emoji_map: Option<HashMap<String, String>>,
    html_generator: HTMLGenerator,
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
        let html_generator = HTMLGenerator::new(tera);
        info!("PostProcesser initialized successfully.");
        Ok(Self {
            api_client,
            storage,
            downloader,
            emoji_map: None,
            html_generator,
            msg_sender,
        })
    }

    pub async fn process(&self, task_id: u64, posts: Vec<Post>) -> Result<()> {
        info!("Processing {} posts for task {}.", posts.len(), task_id);
        let pic_definition = get_config()
            .read()
            .map_err(|err| Error::Other(err.to_string()))?
            .picture_definition;
        debug!("Picture definition set to: {pic_definition:?}");
        let pic_metas = self.extract_all_pic_metas(&posts, pic_definition);
        info!("Found {} unique pictures to download.", pic_metas.len());

        stream::iter(pic_metas)
            .map(Ok)
            .try_for_each_concurrent(10, |meta| async move {
                self.download_pic_to_local(task_id, meta).await
            })
            .await?;

        info!("Finished downloading pictures. Processing posts...");
        for mut post in posts {
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
                            .await
                            .unwrap();
                    }
                }
            }
            self.storage.save_post(&post).await?;
        }
        info!("Finished processing posts for task {task_id}.");
        Ok(())
    }

    pub async fn generate_html(
        &self,
        posts: Vec<Post>,
        options: &ExportOptions,
    ) -> Result<HTMLPage> {
        info!("Generating HTML for {} posts.", posts.len());
        let pic_quality = get_config()
            .read()
            .map_err(|e| Error::Other(e.to_string()))?
            .picture_definition;
        debug!("Using picture quality: {pic_quality:?}");
        let pic_metas = self.extract_all_pic_metas(&posts, pic_quality);
        info!(
            "Found {} unique pictures for HTML generation.",
            pic_metas.len()
        );
        let pic_futures = pic_metas
            .into_iter()
            .map(|m| self.load_picture_from_local(m));
        let pics = try_join_all(pic_futures).await?;
        let pics = pics
            .into_iter()
            .filter_map(|p| p.map(TryInto::<HTMLPicture>::try_into))
            .collect::<Result<Vec<_>>>()?;
        debug!("Loaded {} pictures from local storage.", pics.len());
        let content = self.html_generator.generate_page(posts, options)?;
        info!("HTML content generated successfully.");
        Ok(HTMLPage {
            html: content,
            pics,
        })
    }

    fn extract_emoji_urls(&self, text: &str) -> Vec<&str> {
        EMOJI_EXPR
            .find_iter(text)
            .map(|e| e.as_str())
            .flat_map(|e| self.emoji_map.as_ref().map(|m| m.get(e)))
            .filter_map(|i| i.map(|s| s.as_str()))
            .collect()
    }

    fn extract_all_pic_metas(
        &self,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> HashSet<PictureMeta> {
        let mut pic_metas: HashSet<PictureMeta> = posts
            .iter()
            .flat_map(|post| extract_in_post_pic_metas(post, definition))
            .collect();
        let emoji_metas = posts.iter().flat_map(|post| {
            self.extract_emoji_urls(&post.text)
                .into_iter()
                .map(|url| PictureMeta::other(url.to_string()))
        });
        let avatar_metas = posts
            .iter()
            .flat_map(extract_avatar_metas)
            .collect::<Vec<_>>();
        pic_metas.extend(emoji_metas);
        pic_metas.extend(avatar_metas);
        pic_metas
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

    async fn load_picture_from_local(&self, pic_meta: PictureMeta) -> Result<Option<Picture>> {
        Ok(self
            .storage
            .get_picture_blob(pic_meta.url())
            .await?
            .map(|blob| Picture {
                meta: pic_meta,
                blob,
            }))
    }

    #[allow(unused)]
    async fn load_picture_from_local_or_server(
        &self,
        task_id: u64,
        pic_meta: PictureMeta,
    ) -> Result<Picture> {
        if let Some(blob) = self.storage.get_picture_blob(pic_meta.url()).await? {
            Ok(Picture {
                meta: pic_meta,
                blob,
            })
        } else {
            let storage = self.storage.clone();
            let url = pic_meta.url().to_string();
            let (sender, result) = tokio::sync::oneshot::channel();
            let callback = Box::new(
                move |blob| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                    Box::pin(async move {
                        let pic = Picture {
                            meta: pic_meta,
                            blob,
                        };
                        storage.save_picture(&pic).await?;
                        sender.send(pic).map_err(|pic| {
                            Error::Tokio(format!("pic {} send failed", pic.meta.url()))
                        })?;
                        Ok(())
                    })
                },
            );
            self.downloader
                .download_picture(task_id, url, callback)
                .await?;
            Ok(result.await?)
        }
    }

    #[allow(unused)]
    async fn get_pictures(
        &self,
        task_id: u64,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> Result<Vec<Picture>> {
        let pic_metas = self.extract_all_pic_metas(posts, definition);
        let mut pics = Vec::new();
        for metas in pic_metas {
            pics.push(
                self.load_picture_from_local_or_server(task_id, metas)
                    .await?,
            );
        }
        Ok(pics)
    }
}

fn pic_id_to_url<'a>(
    pic_id: &'a str,
    pic_infos: &'a HashMap<String, Value>,
    quality: &'a PictureDefinition,
) -> Option<&'a str> {
    pic_infos
        .get(pic_id)
        .and_then(|v| v[Into::<&str>::into(quality)]["url"].as_str())
}

fn extract_avatar_metas(post: &Post) -> Vec<PictureMeta> {
    let mut res = Vec::new();
    if let Some(user) = post.user.as_ref() {
        let meta = PictureMeta::avatar(user.avatar_hd.to_owned(), user.id);
        res.push(meta)
    }
    if let Some(u) = post
        .retweeted_status
        .as_ref()
        .and_then(|re| re.user.as_ref())
    {
        let meta = PictureMeta::avatar(u.avatar_hd.to_owned(), u.id);
        res.push(meta);
    }
    res
}

fn extract_in_post_pic_metas(post: &Post, definition: PictureDefinition) -> Vec<PictureMeta> {
    process_in_post_pics(post, |id, pic_infos, post| {
        pic_id_to_url(id, pic_infos, &definition)
            .map(|url| PictureMeta::in_post(url.to_string(), post.id))
    })
}

fn process_in_post_pics<T, F>(post: &Post, mut f: F) -> Vec<T>
where
    F: FnMut(&str, &HashMap<String, Value>, &Post) -> Option<T>,
{
    if let Some(retweeted_post) = &post.retweeted_status {
        process_in_post_pics(retweeted_post, f)
    } else if let Some(pic_ids) = post.pic_ids.as_ref()
        && !pic_ids.is_empty()
    {
        let Some(pic_infos) = post.pic_infos.as_ref() else {
            error!(
                "Missing pic_infos while pic_ids exists for post {}",
                post.id
            );
            return Default::default();
        };
        pic_ids
            .iter()
            .filter_map(|id| f(id, pic_infos, post))
            .collect()
    } else {
        Default::default()
    }
}
