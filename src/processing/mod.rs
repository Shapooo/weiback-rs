pub mod html_generator;

use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use serde_json::Value;
use weibosdk_rs::WeiboAPI;

use crate::app::options::TaskOptions;
use crate::error::{Error, Result};
use crate::exporter::{ExportOptions, HTMLPage};
use crate::models::{Picture, PictureDefinition, PictureMeta, Post};
use crate::storage::Storage;
use crate::utils::EMOJI_EXPR;
use html_generator::{HTMLGenerator, create_tera};

#[derive(Debug, Clone)]
pub struct PostProcesser<W: WeiboAPI, S: Storage> {
    api_client: Option<W>,
    storage: S,
    emoji_map: Option<HashMap<String, String>>,
    html_generator: HTMLGenerator,
}

impl<W: WeiboAPI, S: Storage> PostProcesser<W, S> {
    pub fn new(api_client: Option<W>, storage: S) -> Result<Self> {
        let path = std::env::current_exe().unwrap();
        let tera_path = path
            .parent()
            .expect("the executable should have parent, maybe bugs in there")
            .join("templates");
        let tera = create_tera(&tera_path)?;
        Ok(Self {
            api_client,
            storage,
            emoji_map: None,
            html_generator: HTMLGenerator::new(tera),
        })
    }

    pub fn set_client(&mut self, api_client: W) {
        self.api_client = Some(api_client);
        // TODO: fetch emoji
    }

    pub async fn process(&self, posts: Vec<Post>, options: &TaskOptions) -> Result<()> {
        let pic_metas = self.extract_pic_metas(&posts, options.pic_quality);

        for meta in pic_metas {
            self.download_pic_to_local(meta).await?;
        }

        for mut post in posts {
            if post.is_long_text
                && let Ok(long_text) = self
                    .api_client
                    .as_ref()
                    .ok_or(Error::NotLoggedIn)?
                    .get_long_text(post.id)
                    .await
            {
                post.text = long_text
            }
            self.storage.save_post(&post).await?;
        }
        Ok(())
    }

    pub async fn generate_html(&self, posts: &[Post], options: &ExportOptions) -> Result<HTMLPage> {
        let pic_metas = self.extract_pic_metas(posts, options.pic_quality);
        let pic = pic_metas
            .into_iter()
            .map(|m| self.load_picture_from_local(m));
        // TODO: tackle errs
        let (pics, _): (Vec<_>, Vec<_>) = futures::future::join_all(pic)
            .await
            .into_iter()
            .partition_result();
        let pics: Vec<_> = pics.into_iter().filter_map(|p| p).collect();
        let content = self.html_generator.generate_posts(posts, options)?;
        todo!()
    }

    fn extract_emoji_urls(&self, text: &str) -> Vec<&str> {
        EMOJI_EXPR
            .find_iter(text)
            .map(|e| e.as_str())
            .flat_map(|e| self.emoji_map.as_ref().map(|m| m.get(e)))
            .filter_map(|i| i.map(|s| s.as_str()))
            .collect()
    }

    fn extract_pic_metas(
        &self,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> HashSet<PictureMeta> {
        let mut pic_metas: HashSet<PictureMeta> = posts
            .into_iter()
            .flat_map(|post| {
                self.extract_in_post_pic_urls(post, definition)
                    .into_iter()
                    .map(|url| PictureMeta::in_post(url.to_string(), post.id))
            })
            .collect();
        let emoji_metas = posts.into_iter().flat_map(|post| {
            self.extract_emoji_urls(&post.text)
                .into_iter()
                .map(|url| PictureMeta::other(url.to_string()))
        });
        // TODO: get avatars
        pic_metas.extend(emoji_metas);
        pic_metas
    }

    fn extract_in_post_pic_urls<'a>(
        &self,
        post: &'a Post,
        definition: PictureDefinition,
    ) -> Vec<&'a str> {
        let mut pic_vec = post
            .pic_ids
            .as_ref()
            .map(|pic_ids| {
                post.pic_infos
                    .as_ref()
                    .map(|pic_infos| Self::pic_ids_to_urls(pic_ids, pic_infos, definition.into()))
            })
            .flatten()
            .unwrap_or_default();
        if let Some(retweeted_post) = &post.retweeted_status {
            let mut retweeted_pic_vec = self.extract_in_post_pic_urls(retweeted_post, definition);
            pic_vec.append(&mut retweeted_pic_vec);
        }
        pic_vec
    }

    async fn download_pic_to_local(&self, pic_meta: PictureMeta) -> Result<()> {
        if let Some(_) = self.storage.get_picture_blob(pic_meta.url()).await? {
            Ok(())
        } else {
            let blob = self
                .api_client
                .as_ref()
                .ok_or(Error::NotLoggedIn)?
                .download_picture(pic_meta.url())
                .await?;
            let pic = Picture {
                meta: pic_meta,
                blob,
            };
            self.storage.save_picture(&pic).await?;
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
    async fn load_picture_from_local_or_server(&self, pic_meta: PictureMeta) -> Result<Picture> {
        if let Some(blob) = self.storage.get_picture_blob(pic_meta.url()).await? {
            Ok(Picture {
                meta: pic_meta,
                blob,
            })
        } else {
            let blob = self
                .api_client
                .as_ref()
                .ok_or(Error::NotLoggedIn)?
                .download_picture(pic_meta.url())
                .await?;
            let pic = Picture {
                meta: pic_meta,
                blob,
            };
            self.storage.save_picture(&pic).await?;
            Ok(pic)
        }
    }

    async fn get_pictures(
        &self,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> Result<Vec<Picture>> {
        let pic_metas = self.extract_pic_metas(posts, definition);
        let mut pics = Vec::new();
        for metas in pic_metas {
            pics.push(self.load_picture_from_local_or_server(metas).await?);
        }
        Ok(pics)
    }

    fn pic_ids_to_urls<'a>(
        pic_ids: &'a [String],
        pic_infos: &'a HashMap<String, Value>,
        quality: &'a str,
    ) -> Vec<&'a str> {
        pic_ids
            .iter()
            .filter_map(|id| {
                pic_infos
                    .get(id)
                    .map(|v| v[quality]["url"].as_str())
                    .flatten()
            })
            .collect()
    }
}
