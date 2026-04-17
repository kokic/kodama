// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

mod core;
mod document;
mod header;

pub use core::{
    catalog_item, footnote_reference, html_article_inner, html_catalog_block, html_code_block,
    html_typst_figure, html_figure_code, html_footer, html_footer_section, html_header_nav,
    html_inline_typst_span, html_link,
};
pub use document::{html_doc, html_main_script, html_main_style};
pub use header::{html_header, HtmlHeaderArgs};
