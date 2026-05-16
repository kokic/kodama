# Publishing and Maintenance Workflows

## Recommended Authoring Loop

1. Create or edit `.md` and `.typst` sections under the source tree.
2. Run `kodama serve` while authoring.
3. Run `kodama check --strict` before publishing.
4. Run `kodama build` for the final output.
5. Deploy the configured publish output directory.

## Static Hosting

Kodama output is static HTML, CSS, JavaScript, assets, and optional JSON/XML artifacts. Any static hosting provider can serve it.

Use:

```sh
kodama build
```

Then publish the configured `[build].output` directory.

If your host serves the site under a subpath, configure:

```toml
[kodama]
base-url = "/subpath/"
```

If you enable RSS, use an absolute base URL:

```toml
[kodama]
base-url = "https://example.com/"

[publish]
rss = true
```

## Pretty URLs

Enable pretty URLs when your host maps extensionless paths to generated HTML pages:

```toml
[build]
pretty-urls = true
```

For local preview, make sure the configured static server command supports the same URL style.

## Cache and Incremental Builds

Kodama maintains cache data under `.cache`. Normal builds reuse caches and hash checks to avoid unnecessary work. Use:

```sh
kodama build --no-cache
```

when investigating stale output or after making broad environment changes.

Serve mode also keeps an in-memory compile session and uses watcher dirty-path batches to avoid full rebuilds where possible.

## Upgrading Existing Sites

After installing a newer Kodama release, run:

```sh
kodama upgrade
```

This rewrites the configuration into the current shape and syncs the bundled Typst library. To inspect a config upgrade first:

```sh
kodama upgrade config --output Kodama.upgraded.toml
```

To sync only the Typst library:

```sh
kodama upgrade typst-lib
```

## Editor Integration

Generate VS Code snippets:

```sh
kodama snip --section --katex --inline-section
```

Configure edit links separately for publish and serve workflows:

```toml
[build]
edit = "https://example.com/edit/"

[serve]
edit = "vscode://file/"
```

## Troubleshooting

Run `kodama check` when a build fails or generated links look wrong. It catches many graph and content issues before writing output.

Common issues:

- Missing `index` section: add `index.md` or `index.typst`.
- Dangling local link: fix the target path or create the target section.
- Cyclic embed: remove or redesign the embed chain.
- Typst render error: verify Typst is installed and the Typst source compiles independently.
- Include read error: check the resolved include path and permissions.
- RSS base URL error: set `[kodama].base-url` to an absolute `http://` or `https://` URL.
