// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Kokic (@kokic)

// To be compatible with the current tinymist
#let compatibled-target() = {
  if "target" in dictionary(std) { context std.target() } else { "paged" }
}

#let html-font-size = 15.525pt;

/**
 * There are some external inputs:
 *   sys.inputs.path: relative path of the typst file
 *   sys.inputs.random: a random number in 0..INT64_MAX (note, it's a string)
 */

#let repri(r) = if type(r) == str {
  r
} else {
  repr(r)
}

#let paged-metadata-text-color = gray

#let meta(key, value) = {
  context if compatibled-target() != "paged" {
    let v = value
    let attrs = (key: key)

    if type(value) != content {
      v = none
      attrs.insert("value", repri(value))
    }

    html.elem("kodama-meta", v, attrs: attrs)
  } else {
    if key == "title" {
      block(text(size: 1.5em, weight: "black", value))
    } else {
      [#value #text(" · ")]
    }
  }
}

#let embed(url, title, numbering: false, open: true, catalog: true) = {
  context if compatibled-target() != "paged" {
    let v = title
    let attrs = (url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

    if type(title) != content {
      v = none
      attrs.insert("value", repri(title))
    }

    html.elem("kodama-embed", v, attrs: attrs)
  } else {
    block(below: 0.5em, text(size: 1.083em, weight: "black", title))
    block(text(fill: paged-metadata-text-color)[`numbering:` #numbering ~ `open:` #open ~ `toc:` #catalog])
  }
}

#let local(slug, text) = context if compatibled-target() != "paged" {
  html.elem(
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
} else { underline(text) }

#let external(dest, content) = link(dest, underline(content))

#let tex(raw-tex) = "$" + raw-tex.text + "$"

#let subtree(slug, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = context if compatibled-target() != "paged" {
  let attrs = (slug: repri(slug), numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

  if title != none {
    attrs.insert("title", repri(title))
  }
  if taxon != none {
    attrs.insert("taxon", repri(taxon))
  }

  html.elem("kodama-subtree", content, attrs: attrs)
} else {
  block(below: 0.5em)[
    #if taxon != none {
      let taxon = upper(taxon.at(0)) + taxon.slice(1) + "."
      text(size: 1.083em, weight: "black", fill: rgb("735057"), taxon)
    }
    #text(size: 1.083em, weight: "black", title)
    #underline(stroke: (thickness: 0.1em, dash: "dotted"), text(size: 1.083em, fill: rgb("636363"), raw("[" + slug + "]")))
  ]
  content
}

#let local(slug, text) = context if compatibled-target() != "paged" {
  html.elem(
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
} else {
  text
}

/**
 * HTML: SVG formula rendering vertical position adjustment
 */

#let bounded(eq) = text(top-edge: "bounds", bottom-edge: "bounds", eq)
#let to-em(pt) = str(pt / text.size.pt()) + "em"

// a dict that stores the height of equations
#let equations-height-dict = state("eq_height_dict", (:))
#let is-inside-pin = state("inside_pin", false)

#let pin(label) = context {
  let height = here().position().y
  equations-height-dict.update(it => {
    if label in it.keys() or height < 0.000001pt { it } else {
      it.insert(label, height); it
    }
  })
}

#let add-pin(eq) = {
  let label = repr(eq)
  is-inside-pin.update(true)
  $ inline(pin(label)#bounded(eq)) $
  is-inside-pin.update(false)
}

#let kodama(doc) = {
  context if compatibled-target() == "paged" {
    set page(margin: 2em, paper: "iso-b6", height: auto)
    set par(spacing: 1.5em)
    doc
  } else {
    show math.equation: eq => {
      if eq.block {
        if is-inside-pin.get() {
          html.frame(eq)
        } else {
          html.elem("div", attrs: (style: "display: flex; justify-content: center; width: 100%; margin: 1em 0;"), html.frame(eq))
        }
      } else {
        let label = repr(eq)

        if label in equations-height-dict.final().keys() {
          let height = equations-height-dict.final().at(label, default: none)

          equations-height-dict.update(d => {
            d.insert(label, height); d
          })

          let y-length = measure(bounded(eq)).height
          let shift = y-length - height

          box(html.elem("span", attrs: (style: "vertical-align: -" + to-em(shift.pt()) + ";"), html.frame(bounded(eq))))
        } else {
          box(html.frame(add-pin(eq)))
        }
      }
    }
    doc
  }
}

