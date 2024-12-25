use super::{url_action, Handler};
use crate::{
    adjust_name, config, parse_markdown,
    recorder::{Context, Recorder},
    write_and_inline_html_content,
};
use pulldown_cmark::{Tag, TagEnd};

pub struct Embed {}

impl Handler for Embed {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut Recorder) {
        match tag {
            Tag::Link {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                let (url, action) = url_action(dest_url);
                if action == Context::Embed.strify() {
                    recorder.enter(Context::Embed);
                    recorder.push(url.to_string()); // [0]
                }
            }
            Tag::MetadataBlock(_kind) => {
                recorder.enter(Context::Metadata);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        if *tag == TagEnd::Link && recorder.context == Context::Embed {
            let entry_url = recorder.data.get(0).unwrap().as_str();
            let entry_url = config::join_path(&recorder.relative_dir, entry_url);
            let (parent_dir, filename) = crate::config::parent_dir(&entry_url);
            let html_entry = parse_markdown(&parent_dir, &filename);

            // generate html file & inline article
            let mut html_url = adjust_name(&filename, ".md", ".html");
            html_url = crate::config::output_path(&config::join_path(&parent_dir, &html_url));
            let inline_article = write_and_inline_html_content(&html_url, &html_entry);
            // let event = Event::Html(CowStr::Boxed(inline_article.into()));

            let slug = html_entry.get("slug").map_or("[no_slug]", |s| s);
            let title = html_entry.metadata.title().map_or("[no_title]", |s| s);
            let title = recorder.data.get(1).map(|s| s.as_str()).unwrap_or(title);
            recorder.catalog.push((slug.to_string(), title.to_string()));

            recorder.exit();
            return Some(inline_article);
        }
        match tag {
            TagEnd::MetadataBlock(_kind) => recorder.exit(), 
            _ => {}
        }
        None
    }

    fn text(&self, s: &pulldown_cmark::CowStr<'_>, recorder: &mut Recorder) {
        if recorder.context == Context::Embed {
            recorder.push(s.to_string()); // [1]
        }
    }
}
