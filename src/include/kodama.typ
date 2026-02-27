// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Kokic (@kokic)
// Last modified time: 2026/02/28

/**
 * There are some external inputs:
 *   sys.inputs.path: relative path of the typst file
 *   sys.inputs.random: a random number in 0..INT64_MAX (note, it's a string)
 */

#let repri(r) = if type(r) == str { r } else {
  repr(r)
}

// To be compatible with the current tinymist
#let compatibled-target() = {
  if "target" in dictionary(std) { context std.target() } else { "paged" }
}

#let html-font-size = 15.525pt

// paged
#let paged-metadata-text-color = gray
#let small-block-below = 0.65em
#let heading-font-weight = "black"
#let slug-color = gray
#let taxon-color = gray

#let is_preset_key(key) = {
  (
    "title",
    "taxon",
    "parent",
    "page-title",
    "backlinks",
    "transparent-backlinks",
    "references",
    "asref",
    "asback",
    "footer-mode",
  ).contains(key)
}

#let dotted-stroke = (thickness: 0.1em, dash: ("dot", "dot")/* = thickness */)

#let span-slug(slug) = underline(stroke: dotted-stroke, text(size: 1.083em, fill: slug-color, raw("[" + slug + "]")))

#let taxon-upper(taxon) = upper(taxon.at(0)) + taxon.slice(1) + "."

#let metadata(table) = {
  let title = table.at("title", default: "")
  let taxon = table.at("taxon", default: none)

  let table-pairs = table.pairs()
  let custom-pairs = table-pairs.filter(e => not is_preset_key(e.at(0)))

  context if compatibled-target() != "paged" {
    table-pairs.map(e => {
      let value = e.at(1)
      let v = value
      let attrs = (key: e.at(0))

      if type(value) != content {
        v = none
        attrs.insert("value", repri(value))
      }
      html.elem("kodama-meta", v, attrs: attrs)
    }).join()
  } else {
    if taxon != none {
      text(weight: heading-font-weight, fill: taxon-color, size: 1.35em, taxon-upper(taxon))
    }
    block(above: small-block-below, below: small-block-below, text(size: 1.5em, weight: heading-font-weight, title))
    block(custom-pairs.map(e =>
    e.at(1)).join(text(" · ")))
  }
}

#let external(dest, content) = link(dest, underline(content))

/// 
/// - raw-tex (string): raw TeX math source code without delimiters
/// -> string
#let tex(raw-tex) = "$" + raw-tex.text + "$"

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
} else { underline(stroke: dotted-stroke, text) }

#let embed(url, title, numbering: false, open: true, catalog: true, display-options: false) = {
  context if compatibled-target() != "paged" {
    let v = title
    let attrs = (url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

    if type(title) != content {
      v = none
      attrs.insert("value", repri(title))
    }

    html.elem("kodama-embed", v, attrs: attrs)
  } else {
    block(below: small-block-below, text(size: 1.083em, weight: heading-font-weight, title))
    if display-options {
      block(text(fill: paged-metadata-text-color)[`numbering:` #numbering ~ `open:` #open ~ `toc:` #catalog])
    }
  }
}

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
  block(below: small-block-below)[
    #if taxon != none {
      text(size: 1.083em, weight: heading-font-weight, fill: taxon-color, taxon-upper(taxon))
    }
    #text(size: 1.083em, weight: heading-font-weight, title)
    #span-slug(slug)
  ]
  content
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

