#let repri(r) = if type(r) == str {
  r
} else {
  repr(r)
}

#let meta(key, value) = if type(value) == content {
  html.elem("kodama", value, attrs: (type: "meta", key: key))
} else {
  html.elem("kodama", none, attrs: (type: "meta", key: key, value: repri(value)))
}
#let embed(url, title, numbering: false, open: true, catalog: true) = if type(title) == content {
  html.elem(
    "kodama",
    title,
    attrs: (type: "embed", url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog)),
  )
} else {
  html.elem(
    "kodama",
    none,
    attrs: (
      type: "embed",
      url: url,
      numbering: repri(numbering),
      open: repri(open),
      catalog: repri(catalog),
      value: repr(i),
    ),
  )
}
#let local(slug, text) = if type(text) == content {
  html.elem("kodama", text, attrs: (type: "local", slug: slug))
} else {
  html.elem("kodama", attrs: (type: "local", slug: slug, value: repri(text)))
}
#let local-in-meta(slug, text) = "[" + text + "](" + slug + ")"

#let template(it) = {
  show: html.elem.with("html")

  it
}
