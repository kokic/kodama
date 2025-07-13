// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

#[derive(Debug, PartialEq)]
pub enum State {
    /// Writable state
    None,

    /// Embeded contents
    Embed,

    /// Shared for inline typst
    Shared,

    /// Export typst to HTML fragment
    Html,

    /// Inline typst
    InlineTypst,

    /// `display: inline`
    ImageSpan,

    /// `display: block; text-align: center`
    ImageBlock,

    /// `ImageBlock` with `<details>` code
    ImageCode,

    Metadata,
    LocalLink,
    ExternalLink,
}

impl State {
    pub const fn strify(&self) -> &'static str {
        match self {
            State::None => "none",
            State::Embed => "embed",
            State::Shared => "shared",
            State::Html => "html",
            State::InlineTypst => "inline",
            State::ImageSpan => "span",
            State::ImageBlock => "block",
            State::ImageCode => "code",
            State::Metadata => "metadata",
            State::LocalLink => "local",       // style class name
            State::ExternalLink => "external", // style class name
        }
    }
}
