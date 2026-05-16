# Content Authoring

Kodama turns each source file into one or more sections. A section is the unit that receives metadata, a slug, an HTML page, graph relationships, backlinks, references, and optional footer content.

## Source Files and Slugs

The configured source tree defaults to `trees/`. Files ending in `.md` and `.typst` are section sources.

The section slug is derived from the source path without the extension. For example:

- `trees/index.md` becomes `index`.
- `trees/notes/alice.typst` becomes `notes/alice`.

An `index` section is strongly recommended. If it is missing, Kodama can still compile other sections, but validation and builds will warn because navigation defaults normally point back to `index`.

## Markdown Sections

Markdown sections use a YAML-style metadata block:

```md
---
title: Alice
taxon: note
date: 2026-05-16
---

Alice links to [Bob](./bob).
```

Markdown is parsed with support for common extensions, including tables, task lists, footnotes, math, strikethrough, definition lists, GitHub-flavored Markdown, smart punctuation, and heading attributes.

Raw HTML is filtered by default. Enable it only when you trust the authored content:

```toml
[build]
allow-unsafe-html = true
```

## Typst Sections

Typst sections should import and apply Kodama's Typst library:

```typst
#import "_lib/kodama.typ": *

#show: kodama

#metadata((
  "title": "Alice",
  "taxon": "remark",
  "date": "2026-05-16",
))

Content written in Typst.
```

Kodama compiles Typst through the installed Typst command and reads the generated HTML structure to extract metadata, local links, embeds, and subtrees.

## Metadata

Important metadata fields:

- `title`: rich section title displayed in the article header and links.
- `page-title`: plain browser/page title override. Defaults to the plain-text `title`.
- `taxon`: display category such as `definition`, `remark`, or `example`.
- `data-taxon`: plain taxonomy attribute override. Usually auto-derived from `taxon`.
- `date`: commonly used for sorting and RSS publication dates.
- `parent`: explicit parent section slug for previous-level navigation.
- `backlinks`: `true` or `false`; controls whether this section receives backlinks.
- `transparent-backlinks`: `true` or `false`; displays backlinks even when embedded, except in footer contexts.
- `references`: `true` or `false`; controls whether referenced sections appear in this section's footer.
- `collect`: `true` or `false`; marks a page as a collection page and excludes it from the RSS item list.
- `asref`: `true` or `false`; controls whether the section is treated as a reference target.
- `asback`: `true` or `false`; controls whether the section contributes backlinks.
- `footer-mode`: `embed` or `link`; overrides footer rendering for this section.
- `footer-sort-by`: metadata key used to sort footer entries for this section.

Custom metadata keys are preserved in the metadata index and can be used for project-specific workflows.

## Local Links

Markdown local links become Kodama local references:

```md
[Bob](./bob)
[A nested note](../chapter/note.md)
```

Kodama resolves relative paths from the current section, strips `.md` from local link targets, and validates dangling local links in `kodama check`. Local links can create references and backlinks depending on target metadata and global defaults.

Allowed external schemes are `http`, `https`, `ftp`, and `mailto`. Unsafe schemes such as `javascript`, `vbscript`, `data`, and `file` are downgraded to text.

## Embeds

Use the `#:embed` action to embed another section:

```md
[Embedded Bob](./bob#:embed)
```

The link text becomes the embedded title override. A prefix in the link text controls embed behavior:

- `+`: enable numbering.
- `-`: render details closed by default.
- `.`: omit the embedded section from the catalog/table of contents.

Prefixes can be combined:

```md
[+.-Custom title](./bob#:embed)
```

Typst sources can call the Kodama library's embed helper to produce the same structure.

## Subtrees

Subtrees let one source file define additional sections inline. Markdown supports semantic HTML-like tags:

```md
<definition slug="groups" title="Groups" taxon="definition">
A group is a set with an associative operation, identity, and inverses.
</definition>
```

Supported subtree tags include `block`, `exegesis`, `definition`, `proposition`, `remark`, `conjecture`, `postulate`, `claim`, `observation`, `fact`, `hypothesis`, `axiom`, `lemma`, `theorem`, `corollary`, `example`, and `proof`.

Subtree attributes include:

- `slug`: explicit subtree slug. Without it, Kodama generates an anonymous internal slug.
- `title`: default title for the generated section.
- `taxon`: default taxon for the generated section.
- `numbering`: boolean-like option for numbering.
- `open`: boolean-like option for default details state.
- `catalog`: boolean-like option for catalog inclusion.

Boolean-like subtree options accept missing or `auto` as default, `false`, `0`, or `none` as false, and other present values as true.

Typst sections support the corresponding subtree helpers through the Kodama Typst library.

## Assets

Put static files in the configured assets directory, usually `assets/`. Kodama copies that directory into the output directory during build and serve.

Links that point into the assets root are treated as asset links:

```md
[Diagram](/assets/diagram.svg)
```

## Including Files in Markdown

Use the `#:include` action to include a file as an escaped code block:

```md
[rust](./examples/demo.rs#:include)
```

The link text is used as the language tag. If no text is supplied, `plain` is used.

## Typst Rendering from Markdown

Kodama supports Typst-driven rendering actions from Markdown links:

- Inline Typst links use the `inline` action family.
- `#:html` compiles a Typst file to inline HTML.
- `#:block` renders a Typst file as a block figure.
- `#:span` renders a Typst file as an inline/span figure.
- `#:code` renders a Typst figure with source code.
- `#:shared` imports shared Typst definitions for later inline Typst snippets.

Inline Typst actions may include modifiers such as `math` to wrap content in Typst math mode.

## References and Backlinks

Kodama builds a graph from embeds and local links.

- Embedding a section establishes a parent relationship unless overridden with `parent`.
- Local links can become references when the target section is considered a reference.
- Backlinks are generated for target sections that allow backlinks, from source sections that are allowed to contribute backlinks.
- Footers can show references and backlinks as embedded content or compact links depending on `footer-mode`.

This model is useful for mathematical notes, research forests, documentation gardens, and any site where a short article may be reused inside larger pages while still keeping an independent URL.
