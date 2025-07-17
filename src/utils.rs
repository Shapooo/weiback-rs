use crate::error::{Error, Result};
use anyhow::Context;

macro_rules! here {
    () => {
        concat!("at ", file!(), " line ", line!(), " column ", column!())
    };
}

pub fn pic_url_to_file(url: &str) -> Result<&str> {
    Ok(url
        .rsplit('/')
        .next()
        .ok_or(Error::Other(format!("not a valid picture url: {url}")))
        .context(here!())?
        .split('?')
        .next()
        .ok_or(Error::Other(format!("not a valid picture url: {url}")))
        .context(here!())?)
}

pub fn pic_url_to_id(url: &str) -> Result<&str> {
    let file = pic_url_to_file(url)?;
    let i = file
        .rfind('.')
        .ok_or(Error::Other(format!("not a valid picture url: {url}")))
        .context(here!())?;
    Ok(&file[..i])
}

pub fn strip_url_queries(url: &str) -> &str {
    url.split('?').next().unwrap_or_default()
}
