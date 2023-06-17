pub fn pic_url_to_file(url: &str) -> String {
    url.rsplit('/')
        .next()
        .expect("it is not a valid picture url")
        .split('?')
        .next()
        .expect("it is not a valid picture url")
        .into()
}

pub fn pic_url_to_id(url: &str) -> String {
    let mut file = pic_url_to_file(url);
    let i = file.rfind('.').expect("it is not a valid picture url");
    file.truncate(i);
    file
}

#[cfg(test)]
mod utils_test {
    #[test]
    fn pic_url_to_file_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = super::pic_url_to_file(a);
        dbg!(res);
    }

    #[test]
    fn pic_url_to_id_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = super::pic_url_to_id(a);
        dbg!(res);
    }
}
