use super::*;
use crate::slug::Slug;

#[test]
pub fn test_table_td() {
    let source = "| a | b |\n| - | - |\n| c | d |";
    let mocked_slug = Slug::new("-");

    let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
    let events = Footnote::process(events, mocked_slug);
    let events = Figure::process(events);
    let events = TypstImage::process(events, mocked_slug);
    let events = TextElaborator::process(events);
    let events = Embed::process(events, mocked_slug);

    let content = normalize_html_content(to_contents(events));
    assert_eq!(content.as_str().unwrap(), "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n<tr><td>c</td><td>d</td></tr>\n</tbody></table>\n");
}

#[test]
pub fn test_code_block() {
    let source = "```rs\nlet x = 1;\n```";
    let mocked_slug = Slug::new("-");

    let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
    let events = Footnote::process(events, mocked_slug);
    let events = Figure::process(events);
    let events = TypstImage::process(events, mocked_slug);
    let events = TextElaborator::process(events);
    let events = Embed::process(events, mocked_slug);

    let content = normalize_html_content(to_contents(events));
    assert_eq!(
        content.as_str().unwrap(),
        "<pre><code class=\"language-rs\">let x = 1;\n</code></pre>\n"
    );
}

#[test]
pub fn test_reference_link() {
    let source =
        "---\nlink: [Alice][example]\n---\n\n[Bob][example]\n\n[example]: https://example.com";
    let mocked_slug = Slug::new("-");

    let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
    let events = Footnote::process(events, mocked_slug);
    let events = Figure::process(events);
    let events = TypstImage::process(events, mocked_slug);
    let events = TextElaborator::process(events);
    let events = Embed::process(events, mocked_slug);

    let content = normalize_html_content(to_contents(events));
    assert_eq!(content.as_str().unwrap(), "<p><span class=\"link external\"><a href=\"https://example.com\" title=\"Bob [https://example.com]\">Bob</a></span></p>\n");
}

#[test]
pub fn test_parse_spanned_markdown_wraps_cjk_text() {
    let content = parse_spanned_markdown("hello \u{4e2d}\u{6587} world", Slug::new("-"));
    assert_eq!(
        content.as_str().unwrap(),
        "hello <span lang=\"zh\">\u{4e2d}\u{6587}</span> world"
    );
}

#[test]
pub fn test_parse_spanned_markdown_escapes_raw_html() {
    let content = parse_spanned_markdown("safe <script>alert(1)</script>", Slug::new("-"));
    assert_eq!(
        content.as_str().unwrap(),
        "safe &lt;script&gt;alert(1)&lt;/script&gt;"
    );
}
