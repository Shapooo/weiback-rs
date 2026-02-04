use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use log::error;
use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;

use crate::error::{Error, Result};
use crate::models::{
    MixMediaInfoItem, PicInfoDetail, PicInfoItem, PictureDefinition, PictureMeta, Post,
};

#[allow(unused_macros)]
macro_rules! here {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}

pub static NEWLINE_EXPR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
pub static URL_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(http|https)://[a-zA-Z0-9$%&~_#/.\-:=,?]{5,280}")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static AT_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@[\u4e00-\u9fa5|\uE7C7-\uE7F3|\w_\-·]+")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static EMOJI_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\[.*?\])")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static EMAIL_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static TOPIC_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"#([^#]+)#")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});

pub fn url_to_db_key(url: &Url) -> Url {
    let mut url = url.to_owned();
    url.set_fragment(None);
    url.set_query(None);
    url
}

pub fn url_to_path(url: &Url) -> PathBuf {
    Path::new(url.host_str().expect("host cannot be none")).join(
        url.path()
            .strip_prefix("/")
            .expect("url path start with `/'"),
    )
}

pub fn url_to_filename(url: &Url) -> Result<String> {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .and_then(|name| {
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .ok_or_else(|| Error::FormatError(format!("no filename in url: {url}")))
}

pub fn pic_url_to_id(url: &Url) -> Result<String> {
    let file_path = url_to_path(url);
    file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .ok_or_else(|| Error::FormatError(format!("not a valid picture url: {url}")))
}

pub fn make_resource_dir_name(page_name: &str) -> String {
    page_name.to_string() + "_files"
}

pub fn make_html_file_name(page_name: &str) -> String {
    page_name.to_string() + ".html"
}

pub fn make_page_name(task_name: &str, index: i32) -> String {
    format!("{task_name}-{index}")
}

pub fn extract_all_pic_metas(
    posts: &[Post],
    definition: PictureDefinition,
    emoji_map: Option<&HashMap<String, Url>>,
) -> HashSet<PictureMeta> {
    let mut pic_metas: HashSet<PictureMeta> = posts
        .iter()
        .flat_map(|post| extract_in_post_pic_metas(post, definition))
        .collect();
    let emoji_metas = posts.iter().flat_map(|post| {
        extract_emoji_urls(&post.text, emoji_map)
            .into_iter()
            .filter_map(|url| {
                PictureMeta::other(url)
                    .map_err(|e| error!("cannot parse {url} {e}"))
                    .ok()
            })
    });
    let avatar_metas = posts
        .iter()
        .flat_map(extract_avatar_metas)
        .collect::<Vec<_>>();
    let hyperlink_pic_metas = posts
        .iter()
        .flat_map(|post| extract_hyperlink_pic_metas(post, definition))
        .collect::<Vec<_>>();
    pic_metas.extend(emoji_metas);
    pic_metas.extend(avatar_metas);
    pic_metas.extend(hyperlink_pic_metas);
    pic_metas
}

pub fn def_to_pic_info_detail(
    pic_info_item: &PicInfoItem,
    quality: PictureDefinition,
) -> &PicInfoDetail {
    match quality {
        PictureDefinition::Thumbnail => &pic_info_item.thumbnail,
        PictureDefinition::Bmiddle => &pic_info_item.bmiddle,
        PictureDefinition::Large => &pic_info_item.large,
        PictureDefinition::Original => &pic_info_item.original,
        PictureDefinition::Largest => &pic_info_item.largest,
        PictureDefinition::Mw2000 => &pic_info_item.mw2000,
    }
}

fn extract_hyperlink_pic_metas(post: &Post, definition: PictureDefinition) -> Vec<PictureMeta> {
    let Some(url_struct) = post.url_struct.as_ref() else {
        return Default::default();
    };
    url_struct
        .0
        .iter()
        .filter_map(|i| i.pic_infos.as_ref())
        .filter_map(|p| {
            let url = def_to_pic_info_detail(p, definition).url.as_str();
            PictureMeta::in_post(url, definition, post.id)
                .map_err(|e| error!("cannot parse {url} {e}"))
                .ok()
        })
        .collect()
}

fn extract_emoji_urls<'a>(
    text: &'a str,
    emoji_map: Option<&'a HashMap<String, Url>>,
) -> Vec<&'a str> {
    EMOJI_EXPR
        .find_iter(text)
        .map(|e| e.as_str())
        .flat_map(|e| emoji_map.map(|m| m.get(e)))
        .filter_map(|i| i.map(|s| s.as_str()))
        .collect()
}

pub fn extract_emojis_from_text(text: &str) -> impl Iterator<Item = &str> {
    EMOJI_EXPR.find_iter(text).map(|m| m.as_str())
}

fn extract_avatar_metas(post: &Post) -> Vec<PictureMeta> {
    let mut res = Vec::new();
    if let Some(user) = post.user.as_ref()
        && let Ok(meta) = PictureMeta::avatar(user.avatar_hd.as_str(), user.id)
            .map_err(|e| error!("cannot parse {} {e}", user.avatar_hd.as_str()))
    {
        res.push(meta)
    }
    if let Some(u) = post
        .retweeted_status
        .as_ref()
        .and_then(|re| re.user.as_ref())
        && let Ok(meta) = PictureMeta::avatar(u.avatar_hd.as_str(), u.id)
            .map_err(|e| error!("cannot parse {} {e}", u.avatar_hd.as_str()))
    {
        res.push(meta);
    }
    res
}

pub fn extract_in_post_pic_metas(post: &Post, definition: PictureDefinition) -> Vec<PictureMeta> {
    process_in_post_pics(post, move |pic_info_item| {
        let url = def_to_pic_info_detail(pic_info_item, definition)
            .url
            .as_str();
        PictureMeta::in_post(url, definition, post.id)
            .map_err(|e| error!("cannot parse {url} {e}"))
            .ok()
    })
}

pub fn extract_in_post_pic_paths(
    post: &Post,
    pic_folder: &Path,
    definition: PictureDefinition,
) -> Vec<String> {
    process_in_post_pics(post, |pic_info_item| {
        let url = def_to_pic_info_detail(pic_info_item, definition)
            .url
            .as_str();
        url_to_filename(
            &Url::parse(url).unwrap(), // TODO
        )
        .ok()
        .and_then(|name| pic_folder.join(name).to_str().map(|s| s.to_string()))
    })
}

fn process_in_post_pics<T, F>(post: &Post, f: F) -> Vec<T>
where
    F: Fn(&PicInfoItem) -> Option<T> + Copy,
{
    if let Some(retweeted_post) = &post.retweeted_status {
        return process_in_post_pics(retweeted_post, f);
    }

    if let Some(pic_ids) = post.pic_ids.as_ref() {
        if let Some(pic_infos) = post.pic_infos.as_ref() {
            pic_ids
                .iter()
                .filter_map(|id| pic_infos.get(id).and_then(f))
                .collect()
        } else if let Some(mix_media_info) = post.mix_media_info.as_ref() {
            let map = mix_media_info
                .items
                .iter()
                .filter_map(|i| match i {
                    MixMediaInfoItem::Pic { id, data } => Some((id, data)),
                    _ => None,
                })
                .collect::<HashMap<&String, &Box<PicInfoItem>>>();
            pic_ids
                .iter()
                .filter_map(|id| map.get(id).and_then(|item| f(item.as_ref())))
                .collect()
        } else {
            error!(
                "Missing pic_infos while pic_ids exists for post {}",
                post.id
            );
            Default::default()
        }
    } else {
        Default::default()
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::path::Path;

    use weibosdk_rs::mock::MockClient;

    use crate::api::{EmojiUpdateApi, FavoritesApi, ProfileStatusesApi};
    use crate::mock::MockApi;

    #[test]
    fn test_url_to_filename() {
        assert_eq!(
            url_to_filename(&Url::parse("http://example.com/path/to/file.txt").unwrap()).unwrap(),
            "file.txt"
        );
        assert_eq!(
            url_to_filename(&Url::parse("http://example.com/path/to/file.txt?a=1").unwrap())
                .unwrap(),
            "file.txt"
        );
        assert!(url_to_filename(&Url::parse("http://example.com/").unwrap()).is_err());
        assert!(url_to_filename(&Url::parse("http://example.com").unwrap()).is_err());
    }

    #[test]
    fn test_pic_url_to_id() {
        assert_eq!(
            pic_url_to_id(&Url::parse("http://example.com/path/to/pic.jpg").unwrap()).unwrap(),
            "pic"
        );
        assert_eq!(
            pic_url_to_id(&Url::parse("http://example.com/path/to/pic.jpeg?a=1").unwrap()).unwrap(),
            "pic"
        );
        assert_eq!(
            pic_url_to_id(&Url::parse("http://example.com/path/to/pic.tar.gz").unwrap()).unwrap(),
            "pic.tar"
        );
    }

    #[test]
    fn test_url_to_path() {
        assert_eq!(
            url_to_path(&Url::parse("http://example.com/path/to/file.txt").unwrap()),
            Path::new("example.com/path/to/file.txt")
        );
        assert_eq!(
            url_to_path(&Url::parse("http://example.com/path/to/file.txt?a=1").unwrap()),
            Path::new("example.com/path/to/file.txt")
        );
        assert_eq!(
            url_to_path(&Url::parse("http://example.com").unwrap()),
            Path::new("example.com")
        );
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

    async fn create_posts(api: &MockApi) -> Vec<Post> {
        let mut posts = api.favorites(0).await.unwrap();
        posts.extend(api.profile_statuses(1786055427, 0).await.unwrap());
        posts
    }

    #[tokio::test]
    async fn test_extract_all_pic_metas1() {
        let client = MockClient::new();
        let api = create_mock_api(&client);
        let emoji_map = api.emoji_update().await.unwrap();
        let posts = create_posts(&api).await;
        let set = extract_all_pic_metas(&posts, PictureDefinition::Original, Some(&emoji_map))
            .into_iter()
            .map(|p| pic_url_to_id(p.url()).unwrap().to_owned())
            .collect::<HashSet<String>>();
        let ids = posts
            .into_iter()
            .filter_map(|p| p.pic_ids)
            .flatten()
            .collect::<Vec<_>>();
        for id in ids {
            set.get(&id).unwrap();
        }
    }

    #[tokio::test]
    async fn test_extract_all_pic_metas2() {
        let client = MockClient::new();
        let api = create_mock_api(&client);
        let posts = create_posts(&api).await;
        let emoji_map = api.emoji_update().await.unwrap();

        let metas = extract_all_pic_metas(&posts, PictureDefinition::Large, Some(&emoji_map));

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
        let has_emoji = metas
            .iter()
            .any(|m| m.url().as_str().contains("face.t.sinajs.cn"));

        assert!(has_in_post, "Should extract in-post pictures");
        assert!(has_avatar, "Should extract user avatars");
        assert!(has_emoji, "Should extract emoji pictures");
    }
}
