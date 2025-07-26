use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;

use crate::error::{Error, Result};

macro_rules! here {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}

pub static NEWLINE_EXPR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
pub static URL_EXPR: Lazy<Regex> =
    Lazy::new(|| Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap());
pub static AT_EXPR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@[\\u4e00-\\u9fa5|\\uE7C7-\\uE7F3|\\w_\\-·]+").unwrap());
pub static EMOJI_EXPR: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\\[.*?\\])").unwrap());
pub static EMAIL_EXPR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\\.)+[A-Za-z]{2,6}").unwrap()
});
pub static TOPIC_EXPR: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([^#]+)#").unwrap());

pub fn url_to_path(url: &str) -> Result<String> {
    let url = Url::parse(strip_url_queries(url))?;
    let path = url.path();
    Ok(path.to_string())
}

pub fn url_to_filename(url: &str) -> Result<String> {
    let url = Url::parse(strip_url_queries(url))?;
    url.path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| {
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .ok_or_else(|| Error::Other(format!("no filename in url: {url}")))
}

pub fn pic_url_to_id(url: &str) -> Result<String> {
    let file_name = url_to_filename(url)?;
    let path = Path::new(&file_name);
    if path.extension().is_none() {
        return Err(Error::Other(format!(
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
        .ok_or_else(|| Error::Other(format!("not a valid picture url: {url}")))
}

pub fn strip_url_queries(url: &str) -> &str {
    url.split_once('?').map_or(url, |(base, _query)| base)
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
