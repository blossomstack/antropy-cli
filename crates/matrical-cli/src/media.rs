//! Finds the images and recordings a markdown text links to, so they can be
//! uploaded with it. The server reads links the same way.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

const ALT_CHARS: usize = 200;

/// The name a link target stands for, if it names a file sent with the text
/// rather than something elsewhere.
fn local_name(target: &str) -> Option<&str> {
    let target = target.strip_prefix("./").unwrap_or(target);
    let elsewhere = target.starts_with('/')
        || target.starts_with('#')
        || target
            .split('/')
            .next()
            .is_some_and(|first| first.contains(':'));
    (!elsewhere && !target.is_empty()).then_some(target)
}

/// Every image or recording the text names locally, in order, once each, with its alt.
pub fn references(markdown: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    let mut open: Option<(String, String)> = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                open = Some((dest_url.into_string(), String::new()));
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, alt)) = open.as_mut() {
                    alt.push_str(&t);
                }
            }
            Event::End(TagEnd::Image) => {
                if let Some((target, alt)) = open.take() {
                    if let Some(name) = local_name(&target) {
                        if !found.iter().any(|(seen, _)| seen == name) {
                            let alt = alt.trim().chars().take(ALT_CHARS).collect();
                            found.push((name.to_owned(), alt));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_files_are_found_once_each() {
        let text = "![Chart](./chart.png) ![Call](call.m4a)\n\
                    ![Again](chart.png) ![Web](https://x.test/a.png) ![Root](/a.png)";
        assert_eq!(
            references(text),
            [
                ("chart.png".to_owned(), "Chart".to_owned()),
                ("call.m4a".to_owned(), "Call".to_owned())
            ]
        );
    }
}
