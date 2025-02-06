pub fn to_hash_id(slug: &str) -> String {
    slug.replace("/", "-")
}

/// path to slug
pub fn to_slug(fullname: &str) -> String {
    let mut slug = fullname;
    if fullname.starts_with("/") {
        slug = &slug[1..]
    } else if fullname.starts_with("./") {
        slug = &slug[2..]
    }

    let slug = &slug[0..slug.rfind('.').unwrap_or(slug.len())];
    pretty_path(std::path::Path::new(&slug))
}

pub fn pretty_path(path: &std::path::Path) -> String {
    posix_style(clean_path(path).to_str().unwrap())
}

pub fn posix_style(s: &str) -> String {
    s.replace("\\", "/")
}

fn clean_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut cleaned_path = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                cleaned_path.pop();
            }
            _ => {
                cleaned_path.push(component.as_os_str());
            }
        }
    }
    cleaned_path
}

pub fn adjust_name(path: &str, expect: &str, target: &str) -> String {
    let prefix = if path.ends_with(expect) {
        &path[0..path.len() - expect.len()]
    } else {
        path
    };
    format!("{}{}", prefix, target)
}
