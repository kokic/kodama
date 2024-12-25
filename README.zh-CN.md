
# Kodama

一个 [Typst](https://github.com/typst/typst) 友好的静态站点生成器. 

[[英语说明](./README.md)]

## 特性列表

- 单二进制, [命令行程序](#使用). 

- Typst 内联支持, 将通过用户设备上安装的 Typst 编译并以 SVG 格式嵌入到 HTML 中, 因此所有的 Typst 功能都可用. 对 Typst 书写的行间公式还带有样式优化. 

- 所有 Markdown 编辑器的原生兼容性, 因为 Kodama 处理的语法是标准的 Markdown [^markdown-syntax], 并且在具体设计上 [别出心裁](#嵌入语法). 因此无需任何编辑器插件, 也能轻松书写. 

- 能以 [Jon Sterling 的森林](https://www.jonmsterling.com/foreign-forester-tfmt-000V.xml) 般的方式组织 Markdown 文件. 

- 沿用 Jon Sterling 的术语 ["护林员"](https://www.jonmsterling.com/foreign-forester-index.xml), Kodama 可以被认为是一个 "变种护林员", [此处](#并非护林员) 说明了它们的差异以及这样做的理由. 

## 使用

```
Usage: kodama.exe <COMMAND>

Commands:
  compile  Compiles an input markdown file into HTML format [aliases: c]
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## 嵌入语法

Kodama 目前能够嵌入两种类型的文件, `.md` 和 `.typ`. 前者是为了支持 [Forest 组织内容的方式](https://www.jonmsterling.com/foreign-forester-tfmt-0001.xml). 后者的作用更是显而易见.  

首先, 所有嵌入文件的语法都是标准的 Markdown 链接语法, 这样设计的好处在于, 用户在书写内容时, 所有带有 Markdown 支持的编辑器都能正确跳转到子文件. 

第二, 链接的 `Text` 部分允许为空, 一旦如此, 在生成的目录中就会使用子文件元数据中的标题. 如果 `Text` 部分不为空, 这就会作为嵌入子文件的新标题. 

第三, 复杂的 Typst 文本必须以外部文件的方式书写并嵌入到某个 Markdown 文件中. 此时, 如果图像以块级方式显示, `Text` 的内容将作为插图的说明文字. 

### Markdown 嵌入

```
[title](path/to/file.md#:embed)
```

### Typst 嵌入

#### 段级插图

```
[](path/to/file.typ#:span)
```

#### 块级插图

```
[figure caption](path/to/file.typ#:block)
```

### Typst 内联

最后还有一种特殊的语法, 用于内联 Typst 的行间公式, 从语法上说, 它也是有效的链接声明. 

具体来说, 例如: 

```
[$(dif F) / (dif X)$](inline)
```

这有一个明显的好处, 因为在一定程度上, Typst 和 $\LaTeX$ 的语法有重合的部分. 例如: 

```
[$X^2$](inline)
```

当用户使用编辑器自带的 Markdown 预览时, `X^2` 会被编辑器的预览程序视为 $\LaTeX$ 进而渲染成 $X^2$. 

## 并非护林员

- Forester 处理的是一门类 $\TeX$ 的 DSL, 交换图的绘制经由 Ti*k*Z 来完成, 因此用户的设备需要准备好 $\LaTeX$ 环境. Kodama 选择在遵循 Markdown 语法的前提下, 处理好与 Typst 的兼容关系. 因此, Kodama 可以视为一种 Forester 在其他生态的尝试, 当然, 它们之间的差异仍然很大. 

- Forester 有一个潜在的功能 [^not-sure], 那就是导出 $\LaTeX$ 文件, 这对于习惯 Forest DSL 写作的学术工作者有一些明显的吸引力. Kodama 的潜在用户则关注功能健全的轻写作和现有生态兼容性. 

- Forester 和 Kodama 的共同用户是那些希望尽量把他们所写内容以 HTML 格式发布到互联网上的人. 

## 名称由来

- Kodama (こだま, Echo) 一词可指代日本民间传说中栖息在树木上的灵魂 (Spirit), 即木霊. 本程序希望捕获到 Forest 概念中的精神 (Spirit). 

- 本程序许多关键的地方都使用了回声 `echo` 这个指令. 

- 其他 neta, 此处略去.  

[^markdown-syntax]: Kodama 使用一个名为 [Pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) 的 CommonMark 解析器, 目前开启了 `公式` 和 `Yaml 风格元数据块` 选项. 

[^not-sure]: 当然我并不确定 Jon Sterling 是否真的打算实现这一点. 

