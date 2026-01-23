use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::pin::Pin;

use futures::future::try_join_all;
use log::{debug, info};
use serde_json::{Value, to_value};
use tera::{Context, Tera};
use url::Url;

use crate::api::EmojiUpdateApi;
use crate::config::get_config;
use crate::emoji_map::EmojiMap;
use crate::error::{Error, Result};
use crate::exporter::{HTMLPage, HTMLPicture};
use crate::media_downloader::MediaDownloader;
use crate::models::{Picture, PictureDefinition, PictureMeta, Post, UrlStruct};
use crate::storage::Storage;
use crate::utils::{
    AT_EXPR, EMAIL_EXPR, EMOJI_EXPR, NEWLINE_EXPR, TOPIC_EXPR, URL_EXPR, extract_all_pic_metas,
    extract_in_post_pic_paths, make_resource_dir_name, url_to_filename,
};

pub fn create_tera(template_path: &Path) -> Result<Tera> {
    let mut path = template_path
        .to_str()
        .ok_or(Error::ConfigError(format!(
            "template path in config cannot convert to str: {template_path:?}"
        )))?
        .to_owned();
    path.push_str("/*.html");
    debug!("init tera from template: {path}");
    let mut templates = Tera::new(&path)?;
    templates.autoescape_on(Vec::new());
    Ok(templates)
}

#[derive(Debug, Clone)]
pub struct HTMLGenerator<E: EmojiUpdateApi, S: Storage, D: MediaDownloader> {
    storage: S,
    downloader: D,
    templates: Tera,
    emoji_map: EmojiMap<E>,
}

impl<E: EmojiUpdateApi, S: Storage, D: MediaDownloader> HTMLGenerator<E, S, D> {
    pub fn new(emoji_map: EmojiMap<E>, storage: S, downloader: D, engine: Tera) -> Self {
        Self {
            storage,
            downloader,
            templates: engine,
            emoji_map,
        }
    }

    fn generate_post(
        &self,
        post: Post,
        page_name: &str,
        emoji_map: Option<&HashMap<String, Url>>,
    ) -> Result<String> {
        let pic_folder = make_resource_dir_name(page_name);
        let pic_quality = get_config().read()?.picture_definition;
        let post = post_to_tera_value(post, &pic_folder, pic_quality, emoji_map)?;

        let context = Context::from_value(post)?;
        let html = self.templates.render("post.html", &context)?;
        Ok(html)
    }

    async fn generate_page(&self, posts: Vec<Post>, page_name: &str) -> Result<String> {
        let emoji_map = self.emoji_map.get_or_try_init().await.ok();
        info!("Generating page for {} posts", posts.len());
        let posts_html = posts
            .into_iter()
            .map(|p| self.generate_post(p, page_name, emoji_map))
            .collect::<Result<Vec<_>>>()?;
        let posts_html = posts_html.join("");
        let mut context = Context::new();
        context.insert("html", &posts_html);
        let html = self.templates.render("page.html", &context)?;
        info!("Successfully generated page");
        Ok(html)
    }

    pub async fn generate_html(&self, posts: Vec<Post>, page_name: &str) -> Result<HTMLPage> {
        info!("Generating HTML for {} posts.", posts.len());
        let pic_quality = get_config().read()?.picture_definition;
        let emoji_map = self.emoji_map.get_or_try_init().await.ok();
        debug!("Using picture quality: {pic_quality:?}");
        let pic_metas = extract_all_pic_metas(&posts, pic_quality, emoji_map);
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
        let content = self.generate_page(posts, page_name).await?;
        info!("HTML content generated successfully.");
        Ok(HTMLPage {
            html: content,
            pics,
        })
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
            let url = pic_meta.url().to_owned();
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
                .download_media(task_id, &url, callback)
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
        emoji_map: Option<&HashMap<String, Url>>,
    ) -> Result<Vec<Picture>> {
        let pic_metas = extract_all_pic_metas(posts, definition, emoji_map);
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

fn post_to_tera_value(
    mut post: Post,
    pic_folder: &str,
    pic_quality: PictureDefinition,
    emoji_map: Option<&HashMap<String, Url>>,
) -> Result<Value> {
    let pic_folder = Path::new(pic_folder);
    post.text = trans_text(&post, Path::new(pic_folder), emoji_map)?;
    let ret_resource = if let Some(retweet) = post.retweeted_status.as_mut() {
        retweet.text = trans_text(retweet, pic_folder, emoji_map)?;
        Some((
            extract_in_post_pic_paths(retweet, pic_folder, pic_quality),
            extract_avatar_path(retweet, pic_folder),
        ))
    } else {
        None
    };

    let in_post_pic_paths = extract_in_post_pic_paths(&post, pic_folder, pic_quality);
    let avatar_path = extract_avatar_path(&post, pic_folder);
    let mut post = to_value(post)?;
    post["avatar_path"] = to_value(avatar_path)?;
    post["pic_paths"] = to_value(in_post_pic_paths)?;
    if let Some((pic_paths, avatar_path)) = ret_resource {
        post["retweeted_status"]["avatar_path"] = to_value(avatar_path)?;
        post["retweeted_status"]["pic_paths"] = to_value(pic_paths)?;
    }
    Ok(post)
}

fn trans_text(
    post: &Post,
    pic_folder: &Path,
    emoji_map: Option<&HashMap<String, Url>>,
) -> Result<String> {
    // find all email suffixes
    let emails_suffixes = EMAIL_EXPR
        .find_iter(&post.text)
        .filter_map(|m| AT_EXPR.find(m.as_str()).map(|m| m.as_str()))
        .collect::<HashSet<_>>();

    // convert all '\n' to '<br />' newline tag
    let text = NEWLINE_EXPR.replace_all(&post.text, "<br />");

    // convert all url to hyperlink
    let text = {
        let res = URL_EXPR
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()] + trans_url(post.url_struct.as_ref(), m.as_str()),
                    m.end(),
                )
            });
        res.0 + Borrowed(&text[res.1..])
    };

    // convert all @ to hyperlink, except email suffixes
    let text = {
        let res = AT_EXPR
            .find_iter(&text)
            .filter(|m| !emails_suffixes.contains(m.as_str()))
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + Borrowed(&text[i..m.start()]) + trans_user(&text[m.start()..m.end()]),
                    m.end(),
                )
            });
        res.0 + Borrowed(&text[res.1..])
    };

    // convert all topic to hyperlink
    let text = {
        let res = TOPIC_EXPR
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()] + trans_topic(&text[m.start()..m.end()]),
                    m.end(),
                )
            });
        res.0 + Borrowed(&text[res.1..])
    };

    // convert all emoji mark to emoji pic
    let text = {
        let res = EMOJI_EXPR
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()]
                        + trans_emoji(&text[m.start()..m.end()], pic_folder, emoji_map),
                    m.end(),
                )
            });
        res.0 + Borrowed(&text[res.1..])
    };
    Ok(text.to_string())
}

fn trans_emoji<'a>(
    s: &'a str,
    pic_folder: &'a Path,
    emoji_map: Option<&HashMap<String, Url>>,
) -> Cow<'a, str> {
    let Some(emoji_url) = emoji_map.and_then(|m| m.get(s)) else {
        return s.into();
    };
    let Ok(pic_name) = url_to_filename(emoji_url) else {
        return s.into();
    };
    let pic_path = pic_folder.join(pic_name);
    let Some(pic_path) = pic_path.to_str() else {
        return s.into();
    };
    Borrowed(r#"<img class="bk-emoji" alt=""#)
        + s
        + r#"" title=""#
        + s
        + r#"" src=""#
        + Owned(pic_path.to_owned())
        + r#"" />"#
}

fn trans_user(s: &str) -> Cow<'_, str> {
    Borrowed(r#"<a class="bk-user" href="https://weibo.com/n/"#) + &s[1..] + "\">" + s + "</a>"
}

fn trans_topic(s: &str) -> Cow<'_, str> {
    Borrowed(r#"<a class ="bk-link" href="https://s.weibo.com/weibo?q="#)
        + s
        + r#"" target="_blank">"#
        + s
        + "</a>"
}

fn trans_url<'a>(url_struct: Option<&'a UrlStruct>, url: &'a str) -> Cow<'a, str> {
    let this_struct = url_struct.and_then(|p| p.0.iter().find(|u| u.short_url.as_str() == url));
    let url_title = this_struct
        .map(|u| u.url_title.as_str())
        .unwrap_or("网页链接");
    let url = if let Some(long_url) = this_struct.and_then(|u| u.long_url.as_ref()) {
        long_url.as_str()
    } else {
        url
    };

    Borrowed(r#"<a class="bk-link" target="_blank" href=""#)
        + url
        + "\"><img class=\"bk-icon-link\" src=\"https://h5.sinaimg.cn/upload/2015/09/25/3/\
               timeline_card_small_web_default.png\"/>"
        + url_title
        + "</a>"
}

fn extract_avatar_path(post: &Post, pic_folder: &Path) -> Option<String> {
    post.user
        .as_ref()
        .map(|u| {
            url_to_filename(&u.avatar_hd).and_then(|name| {
                pic_folder
                    .join(&name)
                    .to_str()
                    .ok_or(Error::FormatError(format!(
                        "invalid path {pic_folder:?}/{name}"
                    )))
                    .map(ToString::to_string)
                    .map_err(|e| {
                        log::info!("{e}");
                        e
                    })
            })
        })
        .transpose()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use weibosdk_rs::mock::MockClient;

    use crate::api::{FavoritesApi, ProfileStatusesApi};
    use crate::mock::{MockApi, MockMediaDownloader, MockStorage};

    fn create_test_tera() -> Tera {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("templates");
        create_tera(&path).unwrap()
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

    fn create_generator(api: &MockApi) -> HTMLGenerator<MockApi, MockStorage, MockMediaDownloader> {
        let tera = create_test_tera();
        let storage = MockStorage::new();
        let downloader = MockMediaDownloader::new(true);
        let emoji_map = EmojiMap::new(api.clone());
        HTMLGenerator::new(emoji_map, storage, downloader, tera)
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
        let generator = create_generator(&api);
        let emoji_map = api.emoji_update().await.unwrap();

        for post in posts {
            generator
                .generate_post(post, "test", Some(&emoji_map))
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_generate_post_with_invalid_emoji() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let generator = create_generator(&api);
        for post in posts {
            generator.generate_post(post, "test", None).unwrap();
        }
    }

    #[tokio::test]
    async fn test_generate_page() {
        let client = create_mock_client();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let generator = create_generator(&api);
        generator.generate_page(posts, "test_page").await.unwrap();
    }
}
