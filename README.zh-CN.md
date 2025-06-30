
# Kodama

<img src="https://kokic.github.io/assets/kodama.svg" title="kodama" width=150 /> 

一个 [Typst](https://github.com/typst/typst) 友好的静态 Zettelkästen 站点生成器. 

[[英语说明](./README.md)] [[Demo](https://kokic.github.io)] [[Tutorials]](https://kokic.github.io/tutorials)

## 特性列表

- 单二进制, [命令行程序](#使用). 

- Typst 内联支持, 将通过用户设备上安装的 Typst 编译并以 SVG 格式嵌入到 HTML 中, 所有的 Typst 功能都可用. 对 Typst 书写的行间公式还带有样式优化. 

- 完全自动的明暗主题支持, 对 Typst 输出的公式或彩色图像也一样. 用户能手动调网站样式的任何一个细节, 而无需重新构建 Kodama 工具本身. 

- Markdown 编辑器的原生兼容性. Kodama 处理的是标准 Markdown 文件 [^markdown-syntax], 无需编辑器插件亦可轻松书写. 

- 能以 [Jon Sterling 的森林](https://www.forester-notes.org/index/index.xml) 般的方式组织 Markdown 文件. 

## 使用

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

## 名称由来

Kodama (こだま, Echo) 一词可指代日本民间传说中栖息在树木上的灵魂 (Spirit), 即木霊. 本程序希望捕获到 Forest 概念中的精神 (Spirit). 

[^markdown-syntax]: Kodama 使用一个名为 [Pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) 的 CommonMark 解析器, 目前开启了 `公式` 和 `Yaml 风格元数据块` 选项. 

[^not-sure]: 当然我并不确定 Jon Sterling 是否真的打算实现这一点. 

