/// Example:
/// ```rust
/// let mut s = "a";
/// let html = html_write!(&mut s; br);
/// assert_eq!(html, r#"a<br />"#)
/// ```
macro_rules! html_write {
    // Match a single HTML element with attributes and children
    ($str:expr; $tag:ident $($attr:ident = $val:tt)* { $($inner:tt)* } $($rest:tt)*) => {
        write!($str, "<{}", stringify!($tag)).unwrap();

        // Add attributes
        $(
            write!($str, " {}=\"{}\"", stringify!($attr).replace("_", "-"), $val).unwrap();
        )*

        $str.push('>');

        // Add children (recursively process inner HTML)
        html_write!($str; $($inner)*);

        write!($str, "</{}>", stringify!($tag)).unwrap();

        html_write!($str; $($rest)*);
    };

    // Match a single HTML element without attributes (self-closing tag)
    ($str:expr; $tag:ident $($rest:tt)*) => {{
        write!($str, "<{} />", stringify!($tag)).unwrap();
        html_write!($str; $($rest)*);
    }};

    // Match plain text content
    ($str:expr; $lit:literal $($rest:tt)*) => {{
        write!($str, "{}", $lit).unwrap();
        html_write!($str; $($rest)*);
    }};

    // Match arbitrary expression
    ($str:expr; ($text:expr) $($rest:tt)*) => {{
        write!($str, "{}", $text).unwrap();
        html_write!($str; $($rest)*);
    }};

    // Nothing more, ends here
    ($str:expr;) => {};
}

/// Example:
/// ```rust
/// let value = 1;
/// let id = "some_id";
/// let html = html!(
///     p class="c" id=(id.to_string()) { (value) }
///     br
///     "abc"
/// );
/// assert_eq!(html, r#"<p class="c",id="id">1</p><br />"abc""#)
/// ```
macro_rules! html {
    ($($args:tt)*) => {{
        use ::std::fmt::Write as _;
        use $crate::html_macro::html_write;
        let mut html = String::new();
        html_write!(html; $($args)*);
        html
    }};
}

pub(crate) use {html, html_write};
