
# Kodama

A [Typst](https://github.com/typst/typst)-friendly static site generator.

[[Chinese README](./README.zh-CN.md)]


## Feature List

- Single binary, [command-line program](#usage).

- Typst inline support, which compiles via Typst installed on the user's device and embeds as SVG in HTML, thus all Typst features are available. Additionally, there are style optimizations for inline formulas written in Typst.

- Native compatibility with all Markdown editors, as Kodama processes standard Markdown syntax [^markdown-syntax], and is thoughtfully designed in terms of [embedding syntax](#embedding-syntax). Therefore, no editor plugins are needed for easy writing.

- Organize Markdown files in the manner of [Jon Sterling's Forest](https://www.jonmsterling.com/foreign-forester-tfmt-000V.xml).

- Following Jon Sterling's terminology ["Forester"](https://www.jonmsterling.com/foreign-forester-index.xml), Kodama can be considered a "variant forester", [here](#not-a-forester) explains their differences and the reasons for doing so.

## Usage

```
Usage: kodama.exe <COMMAND>

Commands:
  compile  Compiles an input markdown file into HTML format [aliases: c]
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Embedding Syntax

Kodama currently supports embedding two types of files, `.md` and `.typ`. The former is to support the [Forest way of organizing content](https://www.jonmsterling.com/foreign-forester-tfmt-0001.xml). The latter's role is even more obvious.

First, the syntax for embedding files is standard Markdown link syntax, which has the advantage that when writing content, all editors with Markdown support can correctly jump to sub-files.

Second, the `Text` part of the link is allowed to be empty. If so, the title from the sub-file's metadata will be used in the generated table of contents. If the `Text` part is not empty, it will serve as the new title for the embedded sub-file.

Third, complex Typst text must be written in an external file and embedded into a Markdown file. In this case, if the image is displayed at the block level, the `Text` content will serve as the caption for the illustration.

### Markdown Embedding

```
[title](path/to/file.md#:embed)
```

### Typst Embedding

#### Inline Figure

```
[](path/to/file.typ#:span)
```

#### Block Figure

```
[figure caption](path/to/file.typ#:block)
```

### Typst Inline

Finally, there is a special syntax for inline Typst formulas, which is also a valid link declaration in terms of syntax.

Specifically, for example:

```
[$(dif F) / (dif X)$](inline)
```

This has an obvious advantage, as there is some overlap between Typst and $\LaTeX$ syntax. For example:

```
[$X^2$](inline)
```

When users use the editor's built-in Markdown preview, `X^2` will be treated as $\LaTeX$ by the editor's preview program and rendered as $X^2$.

## Not a Forester

- Forester processes a $\TeX$-like DSL, with diagram drawing done via Ti*k*Z, thus requiring a $\LaTeX$ environment on the user's device. Kodama chooses to handle compatibility with Typst while adhering to Markdown syntax. Therefore, Kodama can be seen as an attempt at Forester in a different ecosystem, although the differences between them remain significant.

- Forester has a potential feature [^not-sure], which is exporting $\LaTeX$ files, which has some obvious appeal for academic workers accustomed to writing in Forest DSL. Kodama's potential users are more concerned with fully functional lightweight writing and compatibility with existing ecosystems.

- Common users of both Forester and Kodama are those who wish to publish their content in HTML format on the internet.

## Name Origin

- Kodama (こだま, Echo) can refer to the spirits inhabiting trees in Japanese folklore, known as 木霊. This program hopes to capture the spirit in the concept of Forest.

- Many key parts of this program use the `echo` command.

- Other neta, omitted here.

[^markdown-syntax]: Kodama uses a CommonMark parser called [Pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark), currently with `formulas` and `Yaml-style metadata blocks` options enabled.

[^not-sure]: Of course, I am not sure if Jon Sterling really intends to implement this.

