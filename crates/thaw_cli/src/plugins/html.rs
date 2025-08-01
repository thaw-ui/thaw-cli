use regex::{Captures, Regex};
use std::collections::HashMap;

#[derive(Debug)]
pub struct IndexHtmlTransformResult {
    pub tags: Vec<HtmlTagDescriptor>,
}

#[derive(Debug)]
pub struct HtmlTagDescriptor {
    pub tag: &'static str,
    pub attrs: HashMap<&'static str, String>,
    pub inject_to: HtmlTagInjectTo,
}

#[derive(Debug, PartialEq)]
pub enum HtmlTagInjectTo {
    HeadPrepend,
}

fn inject_to_head(html: &mut String, tags: Vec<HtmlTagDescriptor>, prepend: bool) {
    if prepend {
        let re = Regex::new("([ \t]*)<head[^>]*>").unwrap();
        *html = re
            .replace(html, move |caps: &Captures| {
                format!(
                    "{}\n{}",
                    &caps[0],
                    serialize_tags(&tags, increment_indent(&caps[1]))
                )
            })
            .into_owned();
    }
}

fn serialize_attrs(attrs: &HashMap<&'static str, String>) -> String {
    attrs
        .iter()
        .map(|(key, value)| format!(r#" {key}="{value}""#))
        .collect()
}

fn serialize_tag(tag: &HtmlTagDescriptor) -> String {
    let HtmlTagDescriptor { tag, attrs, .. } = tag;
    format!("<{tag}{}></{tag}>", serialize_attrs(attrs))
}

fn serialize_tags(tags: &[HtmlTagDescriptor], indent: String) -> String {
    tags.iter()
        .map(|tag| format!("{indent}{}\n", serialize_tag(tag)))
        .collect()
}

fn increment_indent(indent: &str) -> String {
    format!(
        "{indent}{}",
        if indent.starts_with('\t') { "\t" } else { "  " }
    )
}

pub fn apply_html_transform(html: &mut String, tags: Vec<HtmlTagDescriptor>) {
    let mut head_prepend_tags = vec![];

    for tag in tags {
        if tag.inject_to == HtmlTagInjectTo::HeadPrepend {
            head_prepend_tags.push(tag)
        }
    }

    if !head_prepend_tags.is_empty() {
        inject_to_head(html, head_prepend_tags, true);
    }
}

#[test]
fn test_inject_to_head() {
    let mut html = r#"<html><head lang></head><body></body></html>"#.to_string();
    inject_to_head(
        &mut html,
        vec![HtmlTagDescriptor {
            tag: "script",
            attrs: HashMap::from([("type", "module".to_string()), ("src", "/test".to_string())]),
            inject_to: HtmlTagInjectTo::HeadPrepend,
        }],
        true,
    );
    assert_eq!(
        html,
        "<html><head lang>\n  <script type=\"module\" src=\"/test\"></script>\n</head><body></body></html>".to_string()
    );
}
