/*
There are some external inputs:
  sys.inputs.route: relative path of the typst file
  sys.inputs.sha256: sha256 of sys.inputs.route
  sys.inputs.random: a random number in 0..INT64_MAX (note, it's a string)
*/

#let repri(r) = if type(r) == str {
  r
} else {
  repr(r)
}

#let meta(key, value) = {
  let v = value
  let attrs = (key: key)

  if type(value) != content {
    v = none
    attrs.insert("value", repri(value))
  }

  html.elem("kodamameta", v, attrs: attrs)
}

#let embed(url, title, numbering: false, open: true, catalog: true) = {
  let v = title
  let attrs = (url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

  if type(title) != content {
    v = none
    attrs.insert("value", repri(title))
  }

  html.elem("kodamaembed", v, attrs: attrs)
}

#let local(slug, text) = html.elem(
  "span", // Make it an inline element. This is automatically removed by kodama.
  {
    let v = text
    let attrs = (slug: slug)

    if type(text) != content {
      v = none
      attrs.insert("value", repri(text))
    }

    html.elem("kodamalocal", v, attrs: attrs)
  },
)

#let template(it) = {
  show: html.elem.with("html")

  it
}
