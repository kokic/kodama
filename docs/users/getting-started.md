# Getting Started

Kodama is a single-command static site generator for interlinked notes. It reads Markdown and Typst sources, builds HTML pages, copies static assets, and can produce optional machine-readable indexes.

## Prerequisites

- The `kodama` binary.
- Typst installed and available on `PATH` when using `.typst` sections or Typst rendering features from Markdown.
- A local static file server for `kodama serve`. The default serve command uses `miniserve`.

## Create a Site

Create a new site in a new directory:

```sh
kodama new site my-site
```

Create a new site without the Typst support library:

```sh
kodama new site my-site --no-typst
```

Initialize an existing directory:

```sh
kodama init .
```

This creates the default configuration, source tree, assets directory, ignore file, starter index section, and Typst library files unless `--no-typst` is used.

## Create Sections

Create a Typst section:

```sh
kodama new post notes/alice
```

Create a Markdown section:

```sh
kodama new post notes/alice --format .md
```

If the path already has `.typst` or `.md`, Kodama uses that extension and ignores `--format`. Paths are resolved under the configured source tree. Passing a path that already starts with the source tree name is also accepted.

## Build the Site

```sh
kodama build
```

By default this writes the publish output directory configured in `Kodama.toml`, copies the assets directory, writes static runtime files unless they are inlined, and emits:

- `kodama.json`: section metadata index.
- `kodama.graph.json`: parent, reference, and backlink graph.

To force a complete rebuild:

```sh
kodama build --no-cache
```

## Preview Locally

```sh
kodama serve
```

Serve mode builds into the configured serve output directory, starts the configured server command, watches relevant files, and rebuilds on changes. By default, serve mode does not emit `kodama.json` or `kodama.graph.json` unless requested with flags.

Disable live reload:

```sh
kodama serve --disable-reload
```

Print watch dirty-path statistics:

```sh
kodama serve --watch-stats
```

## Validate a Site

```sh
kodama check
```

The check command parses the site and validates the section graph without writing build artifacts. It reports parse errors, missing or duplicate slugs, missing index section warnings, dangling local links, Typst rendering errors encountered during Markdown elaboration, include read errors, and embed graph failures such as cycles.

Treat warnings as failures:

```sh
kodama check --strict
```
