
#[macro_export]
macro_rules! html {
  // Match a single HTML element with attributes and children
  ($tag:ident $($attr:ident = $val:expr),* => $($inner:tt)*) => {{
      let mut html = String::new();
      html.push_str(&format!("<{}", stringify!($tag)));

      // Add attributes
      $(
          html.push_str(&format!(" {}=\"{}\"",
            (stringify!($attr)).replace("_", "-"), $val));
      )*

      html.push('>');

      // Add children (recursively process inner HTML)
      $(
          html.push_str(&html!($inner));
      )*

      html.push_str(&format!("</{}>", stringify!($tag)));
      html
  }};

  // Match a single HTML element without attributes (self-closing tag)
  ($tag:ident) => {{
      format!("<{} />", stringify!($tag))
  }};

  // Match plain text content
  ($text:expr) => {{
      $text.to_string()
  }};
}
