
# Kodama

<img src="https://kokic.github.io/assets/kodama.svg" title="kodama" width=150 /> 

A [Typst](https://github.com/typst/typst)-friendly static Zettelkästen site generator.

[[Chinese README](./README.zh-CN.md)] [[Demo](https://kokic.github.io)] [[Tutorials]](https://kokic.github.io/tutorials)

## Feature List

- Single binary, [command-line program](#usage).

- Typst inline support, which compiles via Typst installed on the user's device and embeds as SVG in HTML, thus all Typst features are available. Additionally, there are style optimizations for inline formulas written in Typst.

- Fully automatic support for light and dark themes, including for formulas or color images output by Typst. Users can also manually adjust any detail of the website style without needing to rebuild the Kodama tool itself.

- Native compatibility with all Markdown editors, as Kodama processes standard Markdown syntax [^markdown-syntax], and is thoughtfully designed in terms of [embedding syntax](#embedding-syntax). Therefore, no editor plugins are needed for easy writing.

- Organize Markdown files in the manner of [Jon Sterling's Forest](https://www.jonmsterling.com/foreign/www.forester-notes.org/tfmt-000V/index.xml).

## Usage

```
Usage: kodama <COMMAND>

Commands:
  compile  Compile current workspace dir to HTMLs [aliases: c]
  watch    Watch files and run build script on changes [aliases: w]
  remove   Remove associated files (hash, entry & HTML) for the given section paths [aliases: rm]
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Name Origin

- Kodama (こだま, Echo) can refer to the spirits inhabiting trees in Japanese folklore, known as 木霊. This program hopes to capture the spirit in the concept of Forest.

- Many key parts of this program use the `echo` command.

- Other neta, omitted here.

[^markdown-syntax]: Kodama uses a CommonMark parser called [Pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark), currently with `formulas` and `Yaml-style metadata blocks` options enabled.

[^not-sure]: Of course, I am not sure if Jon Sterling really intends to implement this.

