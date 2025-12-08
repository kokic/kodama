// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Kokic (@kokic)

/*
There are some external inputs:
  sys.inputs.path: relative path of the typst file
  sys.inputs.random: a random number in 0..INT64_MAX (note, it's a string)
*/

#let target() = {
  if "target" in dictionary(std) { std.target() } else { "paged" }
}

// get the bounding box of an equation
#let bounded(eq) = text(top-edge: "bounds", bottom-edge: "bounds", eq)

// a dict that stores the height of equations
#let eq_height_dict = state("eq_height_dict", (:))

#let inside_pin = state("inside_pin", false)

// when called, retrieves the height of the equation, which is then stored in a state variable
#let pin(label) = context {
  let height = here().position().y
  eq_height_dict.update(it => {
    if label in it.keys() or height < 0.000001pt {
      it
    } else {
      it.insert(label, height)
      it
    }
  })
}

// insert a function call `pin(label)`` at the start of the equation.
#let add_pin(eq) = {
  let label = repr(eq)
  inside_pin.update(true)
  $ inline(pin(label)#bounded(eq)) $
  inside_pin.update(false)
}

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
