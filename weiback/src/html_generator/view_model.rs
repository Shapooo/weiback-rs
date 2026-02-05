use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;
use url::Url;

use crate::error::{Error, Result};
use crate::models::{PictureDefinition, Post, UrlStruct, User};
use crate::utils::{
    AT_EXPR, EMAIL_EXPR, EMOJI_EXPR, NEWLINE_EXPR, TOPIC_EXPR, URL_EXPR, extract_in_post_pic_paths,
    url_to_filename,
};

#[derive(Debug, Serialize)]
pub struct PostView {
    // Fields from Post that are used directly in the template
    user: Option<User>,
    id: i64,
    created_at: String,
    source: Option<String>,
    region_name: Option<String>,

    // Transformed fields
    text: String, // The rendered HTML text

    // Newly generated view-specific fields
    avatar_path: Option<String>,
    pic_paths: Vec<String>,

    // Recursive retweet
    retweeted_status: Option<Box<PostView>>,
}

impl PostView {
    pub fn from_post(
        mut post: Post,
        pic_folder: &str,
        pic_quality: PictureDefinition,
        emoji_map: Option<&HashMap<String, Url>>,
    ) -> Result<Self> {
        let pic_folder_path = Path::new(pic_folder);

        // Handle retweet first if it exists
        let retweeted_status = if let Some(retweet_box) = post.retweeted_status.take() {
            let retweet = *retweet_box;
            Some(Box::new(PostView::from_post(
                retweet,
                pic_folder,
                pic_quality,
                emoji_map,
            )?))
        } else {
            None
        };

        let created_at = post.created_at.to_rfc3339();

        let avatar_path = extract_avatar_path(&post, pic_folder_path);
        let pic_paths = extract_in_post_pic_paths(&post, pic_folder_path, pic_quality);
        let text = trans_text(&post, pic_folder_path, emoji_map)?;

        Ok(PostView {
            id: post.id,
            user: post.user,
            created_at,
            source: post.source,
            region_name: post.region_name,
            text,
            avatar_path,
            pic_paths,
            retweeted_status,
        })
    }
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
    let url = if let Some(long_url) = this_struct.and_then(|u| u.long_url.as_ref())
        && Url::parse(long_url).is_ok()
    {
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
