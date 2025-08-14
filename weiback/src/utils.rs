use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::error;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use url::Url;

use crate::error::{Error, Result};
use crate::models::Post;
use crate::picture::{PictureDefinition, PictureMeta};

#[allow(unused_macros)]
macro_rules! here {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}

pub static NEWLINE_EXPR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
pub static URL_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static AT_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@[\\u4e00-\\u9fa5|\\uE7C7-\\uE7F3|\\w_\\-·]+")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static EMOJI_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\\[.*?\\])")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static EMAIL_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\\.)+[A-Za-z]{2,6}")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});
pub static TOPIC_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"#([^#]+)#")
        .map_err(|e| error!("Regex init failed: {e}"))
        .unwrap()
});

pub fn url_to_path(url: &str) -> Result<String> {
    let url = Url::parse(strip_url_queries(url))?;
    let path = url.path();
    Ok(path.to_string())
}

pub fn url_to_filename(url: &str) -> Result<String> {
    let url = Url::parse(strip_url_queries(url))?;
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

pub fn pic_url_to_id(url: &str) -> Result<String> {
    let file_name = url_to_filename(url)?;
    let path = Path::new(&file_name);
    if path.extension().is_none() {
        return Err(Error::FormatError(format!(
            "no extension in filename of url: {url}"
        )));
    }
    path.file_stem()
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

pub fn strip_url_queries(url: &str) -> &str {
    url.split_once('?').map_or(url, |(base, _query)| base)
}

pub fn page_name_to_resource_dir_name(page_name: &str) -> String {
    page_name.to_string() + "_files"
}

pub fn extract_all_pic_metas(
    posts: &[Post],
    definition: PictureDefinition,
    emoji_map: Option<&HashMap<String, String>>,
) -> HashSet<PictureMeta> {
    let mut pic_metas: HashSet<PictureMeta> = posts
        .iter()
        .flat_map(|post| extract_in_post_pic_metas(post, definition))
        .collect();
    let emoji_metas = posts.iter().flat_map(|post| {
        extract_emoji_urls(&post.text, emoji_map)
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

pub fn pic_id_to_url<'a>(
    pic_id: &'a str,
    pic_infos: &'a HashMap<String, Value>,
    quality: &'a PictureDefinition,
) -> Option<&'a str> {
    pic_infos
        .get(pic_id)
        .and_then(|v| v[Into::<&str>::into(quality)]["url"].as_str())
}

fn extract_emoji_urls<'a>(
    text: &'a str,
    emoji_map: Option<&'a HashMap<String, String>>,
) -> Vec<&'a str> {
    EMOJI_EXPR
        .find_iter(text)
        .map(|e| e.as_str())
        .flat_map(|e| emoji_map.map(|m| m.get(e)))
        .filter_map(|i| i.map(|s| s.as_str()))
        .collect()
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

pub fn process_in_post_pics<T, F>(post: &Post, mut f: F) -> Vec<T>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_url_queries() {
        assert_eq!(
            strip_url_queries("http://example.com/path?a=1&b=2"),
            "http://example.com/path"
        );
        assert_eq!(
            strip_url_queries("http://example.com/path"),
            "http://example.com/path"
        );
        assert_eq!(
            strip_url_queries("http://example.com/path?"),
            "http://example.com/path"
        );
    }

    #[test]
    fn test_url_to_filename() {
        assert_eq!(
            url_to_filename("http://example.com/path/to/file.txt").unwrap(),
            "file.txt"
        );
        assert_eq!(
            url_to_filename("http://example.com/path/to/file.txt?a=1").unwrap(),
            "file.txt"
        );
        assert!(url_to_filename("http://example.com/").is_err());
        assert!(url_to_filename("http://example.com").is_err());
    }

    #[test]
    fn test_pic_url_to_id() {
        assert_eq!(
            pic_url_to_id("http://example.com/path/to/pic.jpg").unwrap(),
            "pic"
        );
        assert_eq!(
            pic_url_to_id("http://example.com/path/to/pic.jpeg?a=1").unwrap(),
            "pic"
        );
        assert_eq!(
            pic_url_to_id("http://example.com/path/to/pic.tar.gz").unwrap(),
            "pic.tar"
        );
        assert!(pic_url_to_id("http://example.com/path/to/pic").is_err());
        assert!(pic_url_to_id("http://example.com/path/to/.jpg").is_err());
    }

    #[test]
    fn test_url_to_path() {
        assert_eq!(
            url_to_path("http://example.com/path/to/file.txt").unwrap(),
            "/path/to/file.txt".to_string()
        );
        assert_eq!(
            url_to_path("http://example.com/path/to/file.txt?a=1").unwrap(),
            "/path/to/file.txt".to_string()
        );
        assert_eq!(url_to_path("http://example.com").unwrap(), "/".to_string());
    }
}
