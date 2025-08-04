use crate::{context::Context, server::middlewares::index_html::dev_html_hook};
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
    pub children: Option<String>,
    pub inject_to: HtmlTagInjectTo,
}

#[derive(Debug, PartialEq)]
pub enum HtmlTagInjectTo {
    HeadPrepend,
    Head,
    Body,
}

static HTML_INJECT: &str = "</html>";
static HEAD_PREPEND_INJECT: &str = "([ \t]*)<head[^>]*>";
static HEAD_INJECT: &str = "([ \t]*)</head>";
static BODY_PREPEND_INJECT: &str = "([ \t]*)<body[^>]*>";
static BODY_INJECT: &str = "([ \t]*)</body>";
static UNARY_TAGS: [&str; 3] = ["link", "meta", "base"];

fn inject_to_head(html: String, tags: Vec<HtmlTagDescriptor>, prepend: bool) -> String {
    if prepend {
        let head_prepend_inject_re = Regex::new(HEAD_PREPEND_INJECT).unwrap();

        // inject as the first element of head
        if head_prepend_inject_re.is_match(&html) {
            return head_prepend_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}\n{}",
                        &caps[0],
                        serialize_tags(&tags, increment_indent(&caps[1]))
                    )
                })
                .into_owned();
        }
    } else {
        let head_inject_re = Regex::new(HEAD_INJECT).unwrap();
        // inject before head close
        if head_inject_re.is_match(&html) {
            return head_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}{}",
                        serialize_tags(&tags, increment_indent(&caps[1])),
                        &caps[0],
                    )
                })
                .into_owned();
        }

        let body_prepend_inject_re = Regex::new(BODY_PREPEND_INJECT).unwrap();
        // inject before the body tag
        if body_prepend_inject_re.is_match(&html) {
            return body_prepend_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}\n{}",
                        serialize_tags(&tags, increment_indent(&caps[1])),
                        &caps[0],
                    )
                })
                .into_owned();
        }
    }

    // if no head tag is present, we prepend the tag for both prepend and append
    prepend_inject_fallback(html, tags)
}

fn inject_to_body(html: String, tags: Vec<HtmlTagDescriptor>, prepend: bool) -> String {
    if prepend {
        let body_prepend_inject_re = Regex::new(BODY_PREPEND_INJECT).unwrap();
        // inject after body open
        if body_prepend_inject_re.is_match(&html) {
            return body_prepend_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}\n{}",
                        &caps[0],
                        serialize_tags(&tags, increment_indent(&caps[1]))
                    )
                })
                .into_owned();
        }
        let head_inject_re = Regex::new(HEAD_INJECT).unwrap();
        // if no there is no body tag, inject after head or fallback to prepend in html
        if head_inject_re.is_match(&html) {
            return head_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}\n{}",
                        &caps[0],
                        serialize_tags(&tags, increment_indent(&caps[1])),
                    )
                })
                .into_owned();
        }

        prepend_inject_fallback(html, tags)
    } else {
        let body_inject_re = Regex::new(BODY_INJECT).unwrap();
        // inject before body close
        if body_inject_re.is_match(&html) {
            return body_inject_re
                .replace(&html, move |caps: &Captures| {
                    format!(
                        "{}{}",
                        serialize_tags(&tags, increment_indent(&caps[1])),
                        &caps[0],
                    )
                })
                .into_owned();
        }

        let html_inject_re = Regex::new(HTML_INJECT).unwrap();
        // if no body tag is present, append to the html tag, or at the end of the file
        if html_inject_re.is_match(&html) {
            return html_inject_re
                .replace(
                    &html,
                    format!("{}\n$&", serialize_tags(&tags, String::new()),),
                )
                .into_owned();
        }

        format!("{html}\n{}", serialize_tags(&tags, String::new()))
    }
}

fn prepend_inject_fallback(html: String, tags: Vec<HtmlTagDescriptor>) -> String {
    let html_prepend_inject_re = Regex::new("([ \t]*)<html[^>]*>").unwrap();
    if html_prepend_inject_re.is_match(&html) {
        return html_prepend_inject_re
            .replace(
                &html,
                format!("$&\n{}", serialize_tags(&tags, String::new()),),
            )
            .into_owned();
    }

    let doctype_prepend_inject_re = Regex::new("<!doctype html>").unwrap();
    if doctype_prepend_inject_re.is_match(&html) {
        return doctype_prepend_inject_re
            .replace(
                &html,
                format!("$&\n{}", serialize_tags(&tags, String::new()),),
            )
            .into_owned();
    }

    format!("{}{}", serialize_tags(&tags, String::new()), html)
}

fn serialize_attrs(attrs: &HashMap<&'static str, String>) -> String {
    attrs
        .iter()
        .map(|(key, value)| format!(r#" {key}="{value}""#))
        .collect()
}

fn serialize_tag(tag: &HtmlTagDescriptor) -> String {
    let HtmlTagDescriptor {
        tag,
        attrs,
        children,
        ..
    } = tag;

    if UNARY_TAGS.contains(tag) {
        format!("<{tag}{}/>", serialize_attrs(attrs),)
    } else {
        format!(
            "<{tag}{}>{}</{tag}>",
            serialize_attrs(attrs),
            children.clone().unwrap_or_default()
        )
    }
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

pub fn apply_html_transform(mut html: String, tags: Vec<HtmlTagDescriptor>) -> String {
    let mut head_prepend_tags = vec![];
    let mut head_tags = vec![];
    let mut body_tags = vec![];

    for tag in tags {
        match tag.inject_to {
            HtmlTagInjectTo::HeadPrepend => {
                head_prepend_tags.push(tag);
            }
            HtmlTagInjectTo::Head => {
                head_tags.push(tag);
            }
            HtmlTagInjectTo::Body => {
                body_tags.push(tag);
            }
        }
    }

    if !head_prepend_tags.is_empty() {
        html = inject_to_head(html, head_prepend_tags, true);
    }
    if !head_tags.is_empty() {
        html = inject_to_head(html, head_tags, false);
    }
    if !body_tags.is_empty() {
        html = inject_to_body(html, body_tags, false);
    }

    html
}

pub struct BuildHtml;

impl BuildHtml {
    pub async fn transform(context: &Context, html: String) -> color_eyre::Result<String> {
        let mut res = Self::main_wasm_hook(context)?;

        if context.serve {
            res.push(dev_html_hook(context).await?);
        }

        let tags = res.into_iter().flat_map(|res| res.tags).collect::<Vec<_>>();

        Ok(apply_html_transform(html, tags))
    }

    fn main_wasm_hook(context: &Context) -> color_eyre::Result<Vec<IndexHtmlTransformResult>> {
        let package_name = context.cargo_package_name()?;
        let assets_path = &context.config.build.assets_dir;
        let js_url = format!("/{assets_path}/{package_name}.js");
        let wasm_url = format!("/{assets_path}/{package_name}_bg.wasm");

        let init_script =
            format!("import init from '{js_url}';await init({{ module_or_path: '{wasm_url}' }})");

        Ok(vec![
            IndexHtmlTransformResult {
                tags: vec![HtmlTagDescriptor {
                    tag: "link",
                    attrs: HashMap::from([("rel", "modulepreload".to_string()), ("href", js_url)]),
                    children: None,
                    inject_to: HtmlTagInjectTo::Head,
                }],
            },
            IndexHtmlTransformResult {
                tags: vec![HtmlTagDescriptor {
                    tag: "link",
                    attrs: HashMap::from([
                        ("rel", "preload".to_string()),
                        ("as", "fetch".to_string()),
                        ("type", "application/wasm".to_string()),
                        ("href", wasm_url),
                    ]),
                    children: None,
                    inject_to: HtmlTagInjectTo::Head,
                }],
            },
            IndexHtmlTransformResult {
                tags: vec![HtmlTagDescriptor {
                    tag: "script",
                    attrs: HashMap::from([("type", "module".to_string())]),
                    children: Some(init_script),
                    inject_to: HtmlTagInjectTo::Body,
                }],
            },
        ])
    }
}

#[test]
fn test_inject_to_head() {
    let html = r#"<html><head lang></head><body></body></html>"#.to_string();
    let html = inject_to_head(
        html,
        vec![HtmlTagDescriptor {
            tag: "script",
            attrs: HashMap::from([("type", "module".to_string()), ("src", "/test".to_string())]),
            children: None,
            inject_to: HtmlTagInjectTo::HeadPrepend,
        }],
        true,
    );
    assert_eq!(
        html,
        "<html><head lang>\n  <script type=\"module\" src=\"/test\"></script>\n</head><body></body></html>".to_string()
    );
}
