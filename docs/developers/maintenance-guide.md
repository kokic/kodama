# Maintenance Guide

This guide describes how to maintain Kodama without depending on concrete source-code locations.

## Design Principles

- Keep parsing, graph resolution, and rendering separate.
- Preserve source-language neutrality after the parser stage.
- Treat slugs as stable public identifiers.
- Prefer structured parsing over ad hoc string rewriting for user-facing syntax.
- Keep build output deterministic so caches, tests, and deployments remain predictable.
- Keep validation behavior at least as strict as build behavior.

## Adding a CLI Feature

When adding or changing a command:

1. Define the command flags with clear help text and conservative defaults.
2. Route the command through the existing mode-aware environment initialization if it reads site configuration.
3. Decide whether the command is allowed to write build output, cache data, snippets, or project files.
4. Add tests for flag resolution and any behavior that differs between build, serve, and check modes.
5. Update user command documentation.

Avoid hidden behavior differences between long-form commands and aliases.

## Adding Configuration

Configuration changes should:

- Use kebab-case TOML keys.
- Have safe defaults.
- Deserialize missing fields successfully for old sites.
- Be included in config upgrade serialization.
- Be reflected in user configuration documentation.
- Have tests for empty config, partial config, and serialization where applicable.

If a setting affects generated URLs, output paths, graph semantics, HTML safety, or cache behavior, add validation or explicit diagnostics.

## Adding Content Syntax

Content syntax changes should be introduced at the parser or processor stage and lowered into the existing unresolved-section model whenever possible.

For Markdown:

- Keep the event-stream model intact.
- Preserve existing processor ordering unless there is a clear reason to change it.
- Ensure new syntax behaves correctly inside metadata, links, subtrees, and footnotes if those contexts are relevant.
- Add malformed-input tests and check-mode diagnostics.

For Typst:

- Prefer extending the Typst library to emit structured markers.
- Parse markers into the same lazy content model used by Markdown.
- Keep metadata and subtree behavior aligned with Markdown semantics.

Implementation checklist:

- Decide whether the syntax belongs to raw source preprocessing, event-stream transformation, Typst marker parsing, or graph resolution.
- Define how the syntax behaves in root sections, nested subtrees, and rich metadata values.
- Define path resolution rules before implementing URL rewriting.
- Decide whether malformed syntax is a parse error, a warning, or plain content.
- Add check-mode coverage if a mistake would otherwise produce broken output.
- Update user authoring documentation with exact syntax examples.

Avoid adding a syntax feature that writes final HTML directly if it could instead become plain content, local link content, embed content, or metadata. Direct HTML should be reserved for final rendering or deliberately escaped include output.

## Graph Semantics

Changes to embeds, local links, references, backlinks, parents, or footer behavior are high-risk because they affect many pages.

Before changing graph behavior, verify:

- Cyclic embed detection still reports useful chains.
- Missing embed targets remain hard errors.
- Dangling local links are reported by check.
- Explicit parents override inferred parents.
- Anonymous subtrees remain hidden from visible graph artifacts.
- Footer sorting remains deterministic.
- JSON graph output continues to describe parent, reference, and backlink relationships accurately.

When changing graph algorithms, test both local and transitive effects:

- A changed embedded child can affect its own page, its parent page, and any page that embeds or links through it.
- A changed linker page can affect the backlink footer of the link target.
- A changed parent can affect header navigation of descendants.
- A changed `asref`, `asback`, `references`, or `backlinks` metadata field can affect pages that do not contain the changed source text directly.
- A changed footer sort key can reorder output without changing the set of referenced slugs.

Graph errors should include the source slug or resolved target slug whenever possible. Missing embed targets should fail builds because output would be structurally incomplete. Dangling local links can remain warnings in validation because users may intentionally draft links before creating targets.

## Output and Compatibility

Generated HTML, JSON, and RSS are public interfaces for users and downstream tooling.

When changing output:

- Keep URL generation consistent with `base-url` and `pretty-urls`.
- Avoid changing `kodama.json` and `kodama.graph.json` shape without a deliberate compatibility decision.
- Keep static asset copying and runtime import behavior compatible with inline and non-inline modes.
- Make RSS changes validate against absolute URL requirements.
- Avoid unnecessary file writes; output hashing is part of the incremental build contract.

Before changing generated output, classify the change:

- Cosmetic HTML changes can be acceptable if page behavior and selectors remain compatible.
- Structural HTML changes may affect themes, custom CSS, snippets, and downstream scraping.
- URL changes are breaking unless gated by configuration.
- JSON shape changes are breaking unless old fields remain available.
- RSS changes should be checked against feed readers and XML escaping rules.

Any output change should be tested with unchanged-output cases as well as changed-output cases so the hash-guarded write behavior remains effective.

## Serve Mode

Serve mode has additional constraints because it combines file watching, an external server process, live reload, caches, and in-memory compiler state.

When changing serve behavior:

- Distinguish source changes from global changes.
- Restart the external server when config changes can affect the server command.
- Reuse in-memory state only when it is still semantically valid.
- Keep live reload optional.
- Preserve the lighter default artifact set unless there is a strong user-facing reason to change it.

Watch behavior should remain conservative. Unknown source-tree dependencies should broaden the dirty set rather than risk stale output. Config changes should restart the external server because the command, output path, edit URL, base URL, and runtime import policy can all change. Asset-only changes should not force a full source parse unless they also affect authored source semantics.

## Caching

Caches are an optimization, not the source of truth.

When changing cached structures:

- Bump or invalidate the cache version.
- Ensure `--no-cache` still produces correct output.
- Keep read-only check behavior independent of publish output.
- Add tests for stale source cleanup when changing source discovery or slug derivation.

Cache-sensitive changes should be reviewed against these cases:

- A source file changes content but not slug.
- A source file changes extension while keeping the same slug.
- A source file is deleted.
- A subtree is renamed, added, removed, or changed from anonymous to named.
- A Typst dependency changes without changing a `.typst` source file.
- An include file changes without changing the Markdown source that includes it.
- A global theme or import changes without changing source files.

If the dependency cannot be tracked precisely, invalidate more than necessary. A slower rebuild is preferable to stale graph or footer output.

## Testing Strategy

Use focused tests around:

- CLI flag resolution.
- Config defaults and upgrades.
- Metadata parsing and text elaboration.
- Path and slug resolution.
- Markdown link classification and safety filtering.
- Subtree extraction, nested subtrees, and anonymous slug allocation.
- Typst marker parsing.
- Graph compilation, cycles, references, backlinks, and parent inference.
- Artifact generation and removal when optional outputs are disabled.
- RSS date handling, sorting, escaping, and URL validation.
- Serve output defaults and watch-change classification.

For broad changes, run the full Rust test suite and perform a smoke build of a demo site.

## Documentation Maintenance

User documentation should change whenever behavior visible through CLI flags, config keys, content syntax, output artifacts, or publishing workflows changes.

Developer documentation should change when the architecture, compiler phases, safety model, cache model, or graph semantics change. Keep it conceptual and avoid referring to exact implementation locations.
