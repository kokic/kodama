# Command Reference

Kodama commands accept the usual `--help` flag. Most commands also have visible aliases for shorter interactive use.

## `kodama new`

Creates site files, config files, or sections.

```sh
kodama new site <path>
kodama new config [path]
kodama new post <path>
```

Aliases:

- `kodama n`
- `kodama new s`
- `kodama new c`
- `kodama new p`

### `new site`

Creates a new directory and writes a default Kodama project into it.

Options:

- `--no-typst`, alias `--nt`: skip Typst library files and create a Markdown starter section.

### `new config`

Writes a default configuration file. The default path is `Kodama.toml`.

### `new post`

Creates a new section under the configured source tree.

Options:

- `--format .typst` or `--format .md`, short `-f`: extension used when the path has no extension.
- `--template <path>`, short `-t`: template file. The placeholder `<FILE_NAME>` is replaced with the new file stem.
- `--config <path>`, short `-c`: configuration file.

## `kodama init`

```sh
kodama init [path]
```

Initializes an existing directory as a Kodama project. The directory must already exist.

Options:

- `--no-typst`, alias `--nt`: skip Typst library files.

Alias: `kodama i`.

## `kodama build`

```sh
kodama build
```

Compiles the current site into the publish output directory.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--verbose`, short `-v`: print build output.
- `--verbose-skip`: print skip output.
- `--no-cache`, alias `--nc`: rebuild all files without using caches.
- `--indexes`: generate `kodama.json`.
- `--no-indexes`: skip `kodama.json`.
- `--graph`: generate `kodama.graph.json`.
- `--no-graph`: skip `kodama.graph.json`.

Alias: `kodama b`.

## `kodama check`

```sh
kodama check
```

Validates the site without generating build artifacts.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--strict`: treat warnings as errors.

Alias: `kodama c`.

## `kodama serve`

```sh
kodama serve
```

Builds the site into the serve output directory, starts the configured static server, watches source/config/theme/assets files, and rebuilds on changes.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--verbose`, short `-v`: print build output.
- `--verbose-skip`: print skip output.
- `--disable-reload`, short `-d`: disable live reload.
- `--watch-stats`, short `-w`: print dirty-path analysis for each watch batch.
- `--indexes`: generate `kodama.json` during serve.
- `--no-indexes`: skip `kodama.json`.
- `--graph`: generate `kodama.graph.json` during serve.
- `--no-graph`: skip `kodama.graph.json`.

Alias: `kodama s`.

## `kodama snip`

```sh
kodama snip --section --katex --inline-section
```

Generates VS Code snippet files in `.vscode/`.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--katex`, short `-k`: generate KaTeX snippets.
- `--section`, short `-s`: generate section reference snippets from current section metadata.
- `--inline-section`, short `-i`: generate inline section snippets.

Section snippets refresh in-memory section indexes before writing snippets.

## `kodama upgrade`

```sh
kodama upgrade
kodama upgrade all
kodama upgrade config
kodama upgrade typst-lib
```

Upgrades configuration shape and/or syncs the bundled Typst library into the configured source tree.

Subcommands:

- `all`, alias `a`: upgrade config and sync Typst library. This is the default when no subcommand is supplied.
- `config`, alias `c`: upgrade config only.
- `typst-lib`, alias `t`: sync `kodama.typ` only.

Options for config upgrades:

- `--config <path>`, short `-c`: source configuration file.
- `--output <path>`, short `-o`: write upgraded config to another path instead of overwriting.

Alias: `kodama u`.
