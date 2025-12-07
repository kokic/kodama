// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under GPL-3.0 license as described in  file LICENSE.
// Authors: Kokic (@kokic)

// To be compatible with the current tinymist
#let compatibled-target() = {
  if "target" in dictionary(std) { std.target() } else { "paged" }
}

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

#show: doc => {
  context if compatibled-target() == "paged" {
    set page(width: auto, height: auto)
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
