// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Kokic (@kokic)

#let html-math-font-size = 15.525pt;

/*
There are some external inputs:
  sys.inputs.path: relative path of the typst file
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

  html.elem("kodama-meta", v, attrs: attrs)
}

#let embed(url, title, numbering: false, open: true, catalog: true) = {
  let v = title
  let attrs = (url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

  if type(title) != content {
    v = none
    attrs.insert("value", repri(title))
  }

  html.elem("kodama-embed", v, attrs: attrs)
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

    html.elem("kodama-local", v, attrs: attrs)
  },
)

#let template(it) = {
  show: html.elem.with("html")

  it
}
