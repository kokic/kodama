# Architecture

Kodama is a Rust command-line application that turns a configured forest of Markdown and Typst sources into static HTML. The central design is a two-stage compiler:

1. Parse every source into shallow, unresolved sections.
2. Resolve the section graph, then write visible pages and optional artifacts.

This split keeps source parsing, graph semantics, and HTML rendering separate enough to support caching, validation, incremental serving, and multiple source languages.

## High-Level Components

Kodama is organized around these responsibilities:

- CLI layer: parses command-line arguments and selects build, check, serve, creation, snippet, and upgrade workflows.
- Environment layer: loads configuration, derives project paths, exposes mode-aware accessors, imports themes and HTML snippets, and manages cache/hash paths.
- Source scanner: discovers source files, records their extension and slug, handles read-only scans for checks, and prepares Typst SVG assets.
- Parser layer: converts Markdown or Typst source files into unresolved sections containing metadata plus plain or lazy content.
- Processing pipeline: transforms Markdown events into Kodama-specific content such as local links, embeds, includes, figures, footnotes, Typst-rendered fragments, and safe HTML.
- Compiler state: resolves embeds and links into a graph of compiled sections, detects cyclic embeds, records parent relationships, references, and backlinks.
- Writer: renders compiled sections into complete HTML documents, footers, catalogs, headers, and RSS-safe content.
- Artifact writer: writes optional metadata and graph JSON, RSS feeds, static runtime files, and copied assets.
- Serve session: maintains in-memory state for local preview and performs incremental rebuilds based on watcher dirty sets.
- Upgrade and scaffolding tools: generate new projects, sections, snippets, config files, and current Typst library files.

## Data Model

The main domain objects are:

- Slug: stable section identifier derived from source paths or subtree declarations.
- Source section: a source file can produce one or more sections.
- Unresolved section: metadata plus content that may contain lazy local links and embeds.
- Compiled section: fully resolved content whose children, references, metadata, options, and footer behavior are known.
- Callback graph: parent and backlink information collected while resolving lazy content.
- Compile state: the full set of compiled sections plus graph state.

Markdown and Typst share the same unresolved-section representation after parsing. This is what allows later graph resolution and writing to be source-language agnostic.

## Source Discovery

Source discovery walks the configured source tree and builds a map from section slug to source extension.

Discovery behavior:

- `README.md` is ignored as a source file.
- Directories whose names begin with `.` or `_` are ignored. This keeps cache directories, helper libraries, and private implementation folders out of the section graph.
- Only supported source extensions are admitted into the workspace.
- The slug is the source-tree-relative path with the extension removed and path separators normalized.
- If two source files would produce the same slug, discovery fails immediately. This prevents ambiguous section ownership such as having both Markdown and Typst sources for the same slug.
- Non-UTF-8 paths discovered during recursive walking are skipped with a warning.
- A missing source tree is not fatal; it produces an empty workspace and a warning.

Typst asset discovery is separate from section discovery. `.typ` files under the source tree are compiled to SVG output assets, while `.typst` files are section sources. During incremental rebuilds, only dirty `.typ` files are recompiled, unless the build is a full scan.

## Path and URL Resolution

Kodama resolves paths in two related but distinct forms:

- Source-relative paths identify files inside the configured source tree.
- Site URLs identify generated HTML pages and copied assets.

Local section links and embeds are resolved relative to the current section slug. Absolute paths that include the source tree root are relocated so authored links can be written either as source-root paths or as site-root paths. Markdown local links strip a `.md` extension after resolution because the generated page URL is slug based rather than source-file based.

Asset links are recognized by checking whether the normalized target path starts with the configured assets root. External links are recognized only for allowed URL schemes, and unsafe schemes are intentionally not emitted as links.

## CLI Workflows

### Build

Build mode initializes the environment in publish mode, ensures cache compatibility, writes or inlines runtime assets, scans sources, syncs Typst SVG assets, parses changed or cached sources, resolves the graph, writes pages, copies assets, and writes optional publish artifacts.

By default, build mode emits metadata and graph JSON. RSS is emitted only when configured.

### Check

Check mode initializes the environment in validation mode and uses read-only source scanning. It parses sections without relying on normal build output, reports diagnostics, validates dangling local links, detects Typst and include errors encountered during Markdown processing, and compiles the section graph to catch graph-level failures.

Check mode does not write build artifacts.

### Serve

Serve mode performs an initial build into the serve output directory, starts the configured static server, watches source, asset, config, theme, and import paths, then decides whether each change batch requires an incremental source rebuild, a global rewrite from memory, or a server restart.

Serve mode defaults metadata and graph JSON off to keep preview output lightweight. It can enable them through output flags.

### Scaffolding and Upgrade

Project creation and initialization generate a config file, source directory, assets directory, starter section, ignore file, and optionally the Typst library. Upgrade workflows deserialize the current config, serialize it into the current schema, and sync the bundled Typst library.

## Parsing Design

Markdown parsing uses a CommonMark event stream with enabled extensions. The event stream passes through processors that:

- Extract YAML-style metadata.
- Convert footnotes and figures.
- Render Typst snippets and Typst-linked figures.
- Elaborate text for language-sensitive output.
- Convert special links into lazy embeds, includes, external links, local links, and asset links.
- Normalize simple plain HTML content.

Markdown subtree extraction happens before the main Markdown parse. Recognized semantic tags are converted into separate sections, and the root source receives lazy embed placeholders that are patched after parsing.

Typst parsing delegates rendering to the installed Typst command. Kodama's Typst library emits structured HTML markers, which are parsed into metadata, local links, embeds, and subtrees. After this point, Typst sections use the same unresolved-section model as Markdown sections.

## Markdown Processing Details

Markdown processing is intentionally streaming after the initial subtree extraction. The main algorithm is:

1. Extract root-level subtrees from the raw Markdown string.
2. Parse the remaining root Markdown into an event stream.
3. Extract metadata from YAML-style metadata blocks.
4. Transform content events through processors for footnotes, figures, Typst rendering, text elaboration, embeds, includes, links, and HTML normalization.
5. Patch subtree placeholders in the root section into real lazy embeds.
6. Repeat extraction and parsing for nested subtree bodies, carrying shared reference definitions from the root source into subtree sources.
7. Reject duplicate generated section slugs.

Metadata values are split into plain and rich fields. Plain fields must compile into text values because later graph and rendering logic reads them as control data. Rich fields, such as `title` and `taxon`, are parsed as inline Markdown-like content and may contain formatting or language elaboration. `taxon` receives display normalization so category names have consistent presentation.

Subtree extraction is a small structural parser rather than full HTML parsing. It scans for recognized semantic opening tags, parses quoted or unquoted attributes, finds the matching closing tag with nesting support, and replaces the subtree body with an internal embed placeholder in the root source. Attributes provide default metadata and embed options. Anonymous subtrees receive generated internal slugs that are stable within the source parse and later hidden from visible graph outputs.

Shared reference definitions are collected from the root Markdown source, excluding fenced code blocks. They are appended to subtree bodies so reference-style Markdown links remain available inside generated subtree sections.

Special Markdown link actions are parsed from the `#:action` suffix. The base URL is kept separate from the action so each processor can decide whether it owns the link. Important actions include:

- `embed`: create lazy embed content for another section.
- `include`: read a file, escape it, and emit a code block.
- `html`: compile a Typst file into inline HTML.
- `block`, `span`, and `code`: render Typst figure variants.
- `shared`: register Typst imports for subsequent inline Typst snippets.
- `inline` and `inline-*`: render inline Typst content, optionally with modifiers such as math mode.

Embed link text has a compact option prefix grammar. Leading `+`, `-`, and `.` toggle numbering, default-open details, and catalog inclusion before the remaining text is used as the title override.

## Typst Processing Details

Typst source processing first asks Typst to render the source into HTML. Kodama then parses structured marker elements emitted by the Kodama Typst library.

The Typst marker parser recognizes metadata, local links, embeds, and subtrees:

- Metadata markers either carry a plain value attribute or nested marker/body HTML that is recursively parsed into rich content.
- Embed markers carry a target URL, optional title, and boolean-like options. Missing or `auto` option values keep defaults; `false`, `0`, and `none` disable an option; other present values enable it.
- Local markers produce lazy local-link content with an already resolved target slug or URL.
- Subtree markers produce additional unresolved sections and insert a lazy embed into the current section.

Typst subtree parsing mirrors Markdown subtree behavior where possible: named subtrees resolve relative to the current slug, anonymous subtrees receive internal generated slugs, and title/taxon attributes become defaults only when the subtree content does not provide those metadata fields itself.

Failures are wrapped with source and slug context because the Typst phase crosses a process boundary and may fail due to syntax errors, missing packages, unavailable fonts, or environmental issues.

## Graph Resolution

The graph compiler starts from `index` when present, then compiles any remaining unlinked sections. During compilation:

- Plain content is copied into the compiled section.
- Embed content fetches the target section, detects cycles, records parent relationships, applies embed options, and can override the embedded title.
- Local links resolve target metadata, produce final HTML links, and may add references and backlinks.
- Metadata values that are rich content are compiled using the same section compiler path so title and taxon HTML remain consistent.

The compiler keeps a visiting stack to detect embed cycles and reports the full cycle chain. Missing embed targets are hard errors. Missing local link targets are surfaced by check diagnostics.

Anonymous internal subtree slugs are normalized so they do not leak into the visible reference/backlink graph.

## Graph Algorithm Details

Graph compilation is depth-first. The compiler maintains:

- A residual set of slugs that have not yet been compiled.
- A compiled map of finished sections.
- A visiting set and compile stack for cycle detection.
- A callback graph for inferred parents and backlinks.

The compiler tries `index` first so sites with a conventional entry point get stable parent inference. It then compiles any residual sections so orphan pages still receive output.

When a lazy embed is encountered:

1. Resolve the target slug relative to the current slug.
2. Fetch and compile the target section recursively if needed.
3. Fail if the target does not exist.
4. Fail if the target is already on the active compile stack, reporting the whole cycle.
5. Record the target's inferred parent as the current slug.
6. Clone the compiled child section into the parent content and apply embed options.
7. Apply an embed title override if the embed supplied one.
8. If the embedded details are open, propagate the embedded section's references into the parent reference set.

When a lazy local link is encountered:

1. Resolve the target slug relative to the current slug.
2. Read the target metadata if available.
3. Use explicit link text when present, otherwise use the target title.
4. Build the final generated URL with the current URL policy.
5. Add the target to the current section's references if the target is considered reference-like.
6. Add the current section to the target's backlink list if both source and target metadata allow it.

After all content is resolved, rich metadata values are compiled through the same unresolved-section machinery. This keeps formatted titles and taxons consistent with normal content and avoids a separate rendering path for metadata.

Parent behavior is intentionally mixed:

- Embeds infer parents.
- `parent` metadata explicitly overrides inferred parents.
- `index` does not point to itself in generated header navigation.
- If a parent cannot be found during writing, the header navigation is skipped with a warning rather than failing the whole build.

## Output Model

For each visible compiled section, the writer creates:

- Header navigation based on the recorded parent or the default index parent.
- Article content with embedded child sections.
- Catalog/table-of-contents content where applicable.
- Footer references and backlinks, rendered either as compact links or embedded content.
- A complete HTML document with configured imports, themes, runtime assets, and page title.

The artifact writer emits optional JSON snapshots for metadata and graph consumers. RSS output is generated in publish mode when enabled and requires an absolute HTTP(S) base URL.

## Rendering and Artifact Details

HTML writing is hash guarded. Before writing a generated page, Kodama compares the new payload with the stored hash for the relative output path. Unchanged pages are skipped, which is important for fast rebuilds and for deployment systems that care about file modification times.

Footer rendering is driven by the section's effective footer mode:

- `link` mode renders compact summaries of referenced or backlink sections.
- `embed` mode renders the referenced or backlink section content recursively into the footer context.

Footer entries are sorted deterministically. Built-in sort keys include slug, date, taxon, and title; arbitrary metadata keys can also be used. Date sorting uses a parser that gives chronological ordering when dates are recognized and stable fallback ordering when they are not.

The graph JSON artifact is a normalized snapshot of compiled graph relationships. Each visible section records its parent, whether the parent was explicitly specified, sorted references, and sorted backlinks. The metadata JSON artifact records visible section metadata only; internal anonymous subtree sections are excluded.

RSS generation uses compiled sections after graph resolution. Collection pages and the index page are excluded from feed items. Item order is reverse date order with slug fallback. Item descriptions are derived by stripping HTML, collapsing whitespace, and truncating to a fixed summary length. Full item content is included as encoded HTML content. Invalid or relative RSS base URLs are rejected before the feed is written.

## Caching and Incrementality

Kodama uses source-entry caches for parsed sections and output hashes to avoid unnecessary writes. A cache version check protects against incompatible cache shape changes.

Incremental builds are driven by dirty paths:

- Dirty source paths map to dirty source slugs.
- The compiler expands affected slugs to include pages impacted by graph dependencies.
- Stale source artifacts are cleaned when a source disappears or changes extension.
- Serve mode can rewrite all pages from memory when global non-source inputs change.

This model favors correctness for graph relationships while still reducing the amount of parsing and writing during local development.

## Cache and Incremental Algorithms

For each source file, Kodama decides whether to parse from source or load a cached unresolved-section entry.

The source modification decision follows this order:

1. If `--no-cache` is active, treat the source as modified.
2. If a dirty set is supplied, only paths in that set are treated as modified, but dirty source paths still update their hash baseline for future cold builds.
3. Without a dirty set, compare the current file hash with the stored hash.

Dirty path expansion is conservative:

- A known dirty `.md` or `.typst` source stays scoped to that source.
- A dirty Typst dependency such as `.typ` or a non-source `.typst` path marks all Typst section sources dirty.
- An unknown file under the source tree, such as an include file, marks all sources dirty because Kodama does not maintain a dependency graph for arbitrary include relationships.

After graph compilation, dirty source slugs are expanded to affected output slugs:

1. Include dirty source slugs.
2. Include embedded descendants whose parent chain starts at a dirty slug. This covers generated subtree sections and embedded ownership.
3. If a changed page contributes backlinks, include the target pages whose backlink lists change.
4. Walk parent and backlink relationships from the affected set until no new affected slugs are found.

If stale slugs are detected because source files disappeared or changed ownership, Kodama writes all visible pages from the current graph to ensure navigation and footers converge to the new state.

Serve mode keeps a compile session in memory. Source changes update the session incrementally when possible. Global changes, such as theme or import changes, can reuse the in-memory graph and rewrite all pages. Config changes trigger a full build and server restart because configuration can affect paths, URL policy, runtime imports, and the external server command.

## Safety Model

Markdown raw HTML is disabled by default. Unsafe link schemes are not emitted as links. Include file contents are escaped before being placed in code blocks. RSS requires an absolute base URL to avoid invalid feed links.

Typst execution is delegated to the user's local Typst installation, so Typst availability and package access are environmental requirements rather than embedded application behavior.

## Check Diagnostics

Check mode is designed to validate behavior that users would otherwise discover after a failed or broken build.

It reports:

- No sections found under the source tree as a hint.
- Missing `index` as a warning.
- Source parse failures as errors.
- Duplicate generated slugs as errors.
- Typst render failures encountered while elaborating Markdown content as errors.
- Include read failures as errors.
- Dangling local links as warnings.
- Graph compilation failures, including cyclic embeds and missing embed targets, as errors.

Strict mode upgrades warnings into command failure. Hints remain informational.
