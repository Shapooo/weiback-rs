use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::error;
use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;

use crate::error::{Error, Result};
use crate::models::{
    HugeInfo, MixMediaInfoItem, PicInfoItem, PictureDefinition, PictureMeta, Post,
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

pub fn pic_url_to_db_key(url: &Url) -> Url {
    let mut url = url.to_owned();
    url.set_fragment(None);
    url.set_query(None);
    url
}

pub fn pic_url_to_path_str(url: &Url) -> String {
    let host = url.host_str().expect("host cannot be none");
    let path = url
        .path()
        .strip_prefix("/")
        .expect("url path start with `/'");
    if path.is_empty() {
        host.to_string()
    } else {
        format!("{}/{}", host, path)
    }
}

pub fn pic_url_to_filename(url: &Url) -> Result<String> {
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
    let file_path_str = pic_url_to_path_str(url);
    Path::new(&file_path_str)
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

pub fn livephoto_video_url_to_path_str(url: &Url) -> Result<String> {
    let queries = url.query_pairs().collect::<HashMap<_, _>>();
    let url = queries
        .get("livephoto")
        .ok_or_else(|| Error::FormatError(format!("unrecognized livephoto url: {url}")))?
        .to_string();
    let url = Url::parse(&url)?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::FormatError(format!("unrecognized livephoto url: {url}")))?;
    let path = url
        .path()
        .strip_prefix("/")
        .expect("url path start with `/'");
    Ok(format!("{}/{}", host, path))
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
        .flat_map(|post| extract_standalone_pic_metas(post, definition))
        .collect();
    let emoji_metas = posts.iter().flat_map(|post| {
        extract_emoji_urls(&post.text, emoji_map).filter_map(|url| {
            PictureMeta::other(url)
                .map_err(|e| error!("cannot parse {url} {e}"))
                .ok()
        })
    });
    let avatar_metas = posts.iter().flat_map(extract_avatar_metas);
    let inline_pic_metas = posts
        .iter()
        .flat_map(|post| extract_inline_pic_metas(post, definition));
    pic_metas.extend(emoji_metas);
    pic_metas.extend(avatar_metas);
    pic_metas.extend(inline_pic_metas);
    pic_metas
}

fn extract_inline_pic_metas(
    post: &Post,
    definition: PictureDefinition,
) -> impl Iterator<Item = PictureMeta> + '_ {
    let outer_id = post.id;
    let outer_text = post.text.as_str();

    let retweet = post.retweeted_status.as_ref();
    let inner_id = retweet.map(|r| r.id);
    let inner_text = retweet.map(|r| r.text.as_str());

    post.url_struct
        .as_ref()
        .into_iter()
        .flat_map(move |url_struct| {
            url_struct.0.iter().filter_map(move |item| {
                let pic_info = item.pic_infos.as_ref()?;
                let short_url = item.short_url.as_str();

                let target_id = if outer_text.contains(short_url) {
                    outer_id
                } else if let Some(in_t) = inner_text
                    && in_t.contains(short_url)
                {
                    inner_id.unwrap() // promised to be Some(_)
                } else {
                    return None;
                };

                let url = pic_info.get_pic_url(definition);
                PictureMeta::attached(url.as_str(), target_id, definition).ok()
            })
        })
}

fn extract_emoji_urls<'a>(
    text: &'a str,
    emoji_map: Option<&'a HashMap<String, Url>>,
) -> impl Iterator<Item = &'a str> {
    EMOJI_EXPR
        .find_iter(text)
        .map(|e| e.as_str())
        .flat_map(move |e| emoji_map.map(|m| m.get(e)))
        .filter_map(|i| i.map(|s| s.as_str()))
}

pub fn extract_emojis_from_text(text: &str) -> impl Iterator<Item = &str> {
    EMOJI_EXPR.find_iter(text).map(|m| m.as_str())
}

fn extract_avatar_metas(post: &Post) -> impl Iterator<Item = PictureMeta> + '_ {
    let current_user_iter = post
        .user
        .as_ref()
        .and_then(|user| {
            PictureMeta::avatar(user.avatar_hd.as_str(), user.id)
                .map_err(|e| error!("cannot parse {} {e}", user.avatar_hd.as_str()))
                .ok()
        })
        .into_iter();

    let retweet_user_iter = post
        .retweeted_status
        .as_ref()
        .and_then(|re| re.user.as_ref())
        .and_then(|u| {
            PictureMeta::avatar(u.avatar_hd.as_str(), u.id)
                .map_err(|e| error!("cannot parse {} {e}", u.avatar_hd.as_str()))
                .ok()
        })
        .into_iter();

    current_user_iter.chain(retweet_user_iter)
}

pub fn generate_standalone_pic_output_paths<'a>(
    post: &'a Post,
    pic_folder: &'a Path,
    definition: PictureDefinition,
) -> impl Iterator<Item = String> + 'a {
    extract_standalone_pic_metas(post, definition).map(|meta| {
        pic_url_to_filename(meta.url())
            .ok()
            .and_then(|name| pic_folder.join(name).to_str().map(|s| s.to_string()))
            .unwrap()
    })
}

fn extract_current_standalone_pic_metas(
    post: &Post,
    definition: PictureDefinition,
) -> impl Iterator<Item = PictureMeta> {
    let post_id = post.id;
    let pic_ids = post.pic_ids.as_ref();
    let pic_infos = post.pic_infos.as_ref();
    let mix_media_info = post.mix_media_info.as_ref();
    let page_info = post.page_info.as_ref();

    let pic_info_handler = move |pic_info_item: &PicInfoItem| {
        let url = pic_info_item.get_pic_url(definition);
        PictureMeta::attached(url.as_str(), post_id, definition)
            .map_err(|e| error!("cannot parse {url} {e}"))
            .ok()
    };
    let huge_info_handler = move |huge_info: &HugeInfo| {
        let url = huge_info.page_pic.as_str();
        PictureMeta::cover(url, post_id)
            .map_err(|e| error!("cannot parse {url} {e}"))
            .ok()
    };

    let pic_infos_iter = pic_ids.into_iter().flat_map(move |ids| {
        ids.iter().filter_map(move |id| {
            pic_infos.and_then(|infos| infos.get(id).and_then(pic_info_handler))
        })
    });

    let mix_media_iter = mix_media_info.into_iter().flat_map(move |mmi| {
        mmi.items.iter().filter_map(move |item| match item {
            MixMediaInfoItem::Pic { data, .. } => pic_info_handler(data),
            MixMediaInfoItem::Video { data, .. } => huge_info_handler(data),
        })
    });

    let page_info_iter = page_info
        .into_iter()
        .flat_map(|p| p.pic_info.as_ref())
        .filter_map(move |p| {
            let url = p.pic_big.url.as_str();
            PictureMeta::cover(url, post_id)
                .map_err(|e| error!("cannot parse {url} {e}"))
                .ok()
        });

    pic_infos_iter.chain(mix_media_iter).chain(page_info_iter)
}

pub fn extract_standalone_pic_metas(
    post: &Post,
    definition: PictureDefinition,
) -> impl Iterator<Item = PictureMeta> {
    // 当存在 retweeted_status 时，只处理 retweeted_status
    let source = post.retweeted_status.as_deref().unwrap_or(post);

    extract_current_standalone_pic_metas(source, definition)
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
            pic_url_to_filename(&Url::parse("http://example.com/path/to/file.txt").unwrap())
                .unwrap(),
            "file.txt"
        );
        assert_eq!(
            pic_url_to_filename(&Url::parse("http://example.com/path/to/file.txt?a=1").unwrap())
                .unwrap(),
            "file.txt"
        );
        assert!(pic_url_to_filename(&Url::parse("http://example.com/").unwrap()).is_err());
        assert!(pic_url_to_filename(&Url::parse("http://example.com").unwrap()).is_err());
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
            pic_url_to_path_str(&Url::parse("http://example.com/path/to/file.txt").unwrap()),
            "example.com/path/to/file.txt".to_string()
        );
        assert_eq!(
            pic_url_to_path_str(&Url::parse("http://example.com/path/to/file.txt?a=1").unwrap()),
            "example.com/path/to/file.txt".to_string()
        );
        assert_eq!(
            pic_url_to_path_str(&Url::parse("http://example.com").unwrap()),
            "example.com".to_string()
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
        posts.extend(
            api.profile_statuses(1786055427, 0, Default::default())
                .await
                .unwrap(),
        );
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

        let has_attached = metas
            .iter()
            .any(|m| matches!(m, PictureMeta::Attached { .. }));
        let has_avatar = metas
            .iter()
            .any(|m| matches!(m, PictureMeta::Avatar { .. }));
        let has_emoji = metas
            .iter()
            .any(|m| m.url().as_str().contains("face.t.sinajs.cn"));

        assert!(has_attached, "Should extract in-post pictures");
        assert!(has_avatar, "Should extract user avatars");
        assert!(has_emoji, "Should extract emoji pictures");
    }
}
