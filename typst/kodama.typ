#let meta(key, value) = html.elem("kodama", value, attrs: (type: "meta", key: key))
#let embed(url, title, numbering: false, open: true, catalog: true) = html.elem(
  "kodama",
  title,
  attrs: (type: "embed", url: url, numbering: repr(numbering), open: repr(open), catalog: repr(catalog)),
)
#let local(slug, text) = html.elem("kodama", text, attrs: (type: "local", slug: slug))

#let template(it) = {
  show: html.elem.with("html")

  it
}