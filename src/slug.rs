pub fn to_id(slug: &str) -> String {
    slug.replace("/", "-")
}

/// path to slug
pub fn to_slug(fullname: &str) -> String {
    let slug = &fullname[0..fullname.rfind('.').unwrap_or(fullname.len())];
    pretty_path(std::path::Path::new(&slug))
}

pub fn pretty_path(path: &std::path::Path) -> String {
    clean_path(path).to_str().unwrap().replace("\\", "/")
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
