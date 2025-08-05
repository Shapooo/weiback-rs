pub mod html_generator;

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use itertools::Itertools;
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
        let path = std::env::current_exe().unwrap();
        let tera_path = path
            .parent()
            .expect("the executable should have parent, maybe bugs in there")
            .join("templates");
        let tera = create_tera(&tera_path)?;
        Ok(Self {
            api_client,
            storage,
            downloader,
            emoji_map: None,
            html_generator: HTMLGenerator::new(tera),
            msg_sender,
        })
    }

    pub async fn process(&self, task_id: u64, posts: Vec<Post>) -> Result<()> {
        let pic_definition = get_config()
            .read()
            .map_err(|err| Error::Other(err.to_string()))?
            .picture_definition;
        let pic_metas = self.extract_all_pic_metas(&posts, pic_definition);

        for meta in pic_metas {
            self.download_pic_to_local(task_id, meta).await?;
        }

        for mut post in posts {
            if post.is_long_text {
                match self.api_client.get_long_text(post.id).await {
                    Ok(long_text) => {
                        post.text = long_text;
                    }
                    Err(e) => {
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
        Ok(())
    }

    pub async fn generate_html(
        &self,
        posts: Vec<Post>,
        options: &ExportOptions,
    ) -> Result<HTMLPage> {
        let pic_metas = self.extract_all_pic_metas(&posts, options.pic_quality);
        let pic = pic_metas
            .into_iter()
            .map(|m| self.load_picture_from_local(m));
        // TODO: tackle errs
        let (pics, _): (Vec<_>, Vec<_>) = futures::future::join_all(pic)
            .await
            .into_iter()
            .partition_result();
        let pics = pics
            .into_iter()
            .filter_map(|p| p.map(TryInto::<HTMLPicture>::try_into))
            .collect::<Result<Vec<_>>>()?;
        let content = self.html_generator.generate_page(posts, options)?;
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
            .into_iter()
            .flat_map(|post| extract_in_post_pic_metas(post, definition))
            .collect();
        let emoji_metas = posts.into_iter().flat_map(|post| {
            self.extract_emoji_urls(&post.text)
                .into_iter()
                .map(|url| PictureMeta::other(url.to_string()))
        });
        let avatar_metas = posts
            .into_iter()
            .flat_map(extract_avatar_metas)
            .collect::<Vec<_>>();
        pic_metas.extend(emoji_metas);
        pic_metas.extend(avatar_metas);
        pic_metas
    }

    async fn download_pic_to_local(&self, task_id: u64, pic_meta: PictureMeta) -> Result<()> {
        if let Some(_) = self.storage.get_picture_blob(pic_meta.url()).await? {
            Ok(())
        } else {
            let storage = self.storage.clone();
            let url = pic_meta.url().to_string();
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

pub(self) fn pic_id_to_url<'a>(
    pic_id: &'a str,
    pic_infos: &'a HashMap<String, Value>,
    quality: &'a PictureDefinition,
) -> Option<&'a str> {
    pic_infos
        .get(pic_id)
        .map(|v| v[Into::<&str>::into(quality)]["url"].as_str())
        .flatten()
}

fn extract_avatar_metas(post: &Post) -> Vec<PictureMeta> {
    let mut res = Vec::new();
    if let Some(user) = post.user.as_ref() {
        let meta = PictureMeta::avatar(user.avatar_hd.to_owned(), user.id);
        res.push(meta)
    }
    post.retweeted_status
        .as_ref()
        .map(|re| re.user.as_ref())
        .flatten()
        .map(|u| {
            let meta = PictureMeta::avatar(u.avatar_hd.to_owned(), u.id);
            res.push(meta);
        });
    res
}

fn extract_in_post_pic_metas<'a>(
    post: &'a Post,
    definition: PictureDefinition,
) -> Vec<PictureMeta> {
    let pic_vec = vec![];
    let mut pic_vec = post
        .pic_ids
        .as_ref()
        .unwrap_or(&pic_vec)
        .iter()
        .filter_map(|id| {
            let pic_infos = HashMap::new();
            let pic_infos = post.pic_infos.as_ref().unwrap_or(&pic_infos);
            pic_id_to_url(id, &pic_infos, &definition)
                .map(|url| PictureMeta::in_post(url.to_string(), post.id))
        })
        .collect::<Vec<_>>();
    // TODO: error handle

    if let Some(retweeted_post) = &post.retweeted_status {
        let mut retweeted_pic_vec = extract_in_post_pic_metas(retweeted_post, definition);
        pic_vec.append(&mut retweeted_pic_vec);
    }
    pic_vec
}
