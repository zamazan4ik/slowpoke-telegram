pub fn is_url(text: &str) -> bool {
    url::Url::parse(text).is_ok()
}
