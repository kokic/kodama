# Configuration Reference

Kodama reads `Kodama.toml` by default. Commands that accept `--config` can point at another file. If the specified config path is not found, Kodama searches from the parent directory in the way used by project commands, so commands can often be run from inside a site.

An empty configuration is valid because every section has defaults.

## `[kodama]`

```toml
[kodama]
trees = "trees"
assets = "assets"
base-url = "/"
theme-lock = false
themes = []
```

- `trees`: source directory for `.md` and `.typst` sections.
- `assets`: static assets directory copied into the output.
- `base-url`: URL prefix used for generated links. Use `/` for root-relative local output, or an absolute `https://.../` URL for RSS publishing.
- `theme-lock`: disables automatic theme switching when true.
- `themes`: list of external theme paths imported into generated pages.

## `[toc]`

```toml
[toc]
placement = "right"
sticky = true
mobile-sticky = true
max-width = "45ex"
```

- `placement`: `left` or `right`.
- `sticky`: whether the table of contents stays fixed while scrolling on larger screens.
- `mobile-sticky`: whether sticky behavior is used on mobile.
- `max-width`: CSS width value for the table of contents.

## `[text]`

```toml
[text]
edit = "[edit]"
toc = "Table of Contents"
references = "References"
backlinks = "Backlinks"
```

These values customize interface labels in generated pages.

## `[build]`

```toml
[build]
typst-root = "trees"
short-slug = false
pretty-urls = false
footer-mode = "link"
footer-sort-by = "slug"
inline-css = false
inline-script = false
allow-unsafe-html = false
asref = false
output = "./publish"
edit = "https://example.com/edit/"
```

- `typst-root`: root directory passed to Typst compilation.
- `short-slug`: enables shortened slug behavior.
- `pretty-urls`: emits links without `.html` suffixes.
- `footer-mode`: `link` for compact footer cards, or `embed` for full embedded footer content.
- `footer-sort-by`: sort key for reference and backlink footer entries. Common values are `slug`, `date`, `taxon`, `title`, or a custom metadata key.
- `inline-css`: embeds Kodama CSS into each page instead of writing `main.css`.
- `inline-script`: embeds Kodama JavaScript into each page instead of writing `main.js`.
- `allow-unsafe-html`: permits raw HTML from Markdown. Keep false for untrusted content.
- `asref`: global default for whether local link targets are treated as references.
- `output`: publish output directory used by `kodama build`.
- `edit`: optional edit URL prefix for generated edit links in publish builds.

## `[serve]`

```toml
[serve]
edit = "vscode://file/"
output = "./.cache/publish"
command = ["miniserve", "<output>", "--index", "index.html", "--pretty-urls"]
```

- `edit`: edit URL prefix for local preview.
- `output`: output directory used by `kodama serve`.
- `command`: server command and arguments. The literal `<output>` is replaced with the serve output directory.

The default command expects `miniserve` to be installed. You can replace it with any static server command that serves the output directory.

## `[publish]`

```toml
[publish]
rss = false
```

- `rss`: when true, `kodama build` writes `feed.xml`.

RSS publishing requires `[kodama].base-url` to be an absolute `http://` or `https://` URL with a host.

## Generated Artifacts

Depending on command flags and configuration, Kodama writes:

- HTML pages for visible sections.
- `main.css` unless CSS is inlined.
- `main.js` unless JavaScript is inlined.
- A copied assets directory.
- `kodama.json` when metadata indexes are enabled.
- `kodama.graph.json` when graph output is enabled.
- `feed.xml` when RSS is enabled for publish builds.

Serve mode defaults index and graph outputs off. Build mode defaults them on.
