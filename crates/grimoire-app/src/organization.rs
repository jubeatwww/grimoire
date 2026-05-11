use std::path::PathBuf;

pub fn build_target_path(
    primary_category: &str,
    circle_or_developer: &str,
    title: &str,
    version: Option<&str>,
    extension: &str,
) -> PathBuf {
    let safe_category = sanitize_segment(primary_category);
    let safe_owner = sanitize_segment(circle_or_developer);
    let safe_title = sanitize_segment(title);
    let safe_extension = extension.trim_start_matches('.');

    let filename = match version {
        Some(version) if !version.trim().is_empty() => {
            format!(
                "{} {}.{}",
                safe_title,
                sanitize_segment(version),
                safe_extension
            )
        }
        _ => format!("{}.{}", safe_title, safe_extension),
    };

    PathBuf::from(safe_category)
        .join(safe_owner)
        .join(&safe_title)
        .join(filename)
}

fn sanitize_segment(value: &str) -> String {
    let replaced = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();
    let trimmed = replaced.trim();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_primary_category_path() {
        let plan = build_target_path("RPG", "Alicesoft", "Sample Game", Some("v1.02"), "zip");
        assert_eq!(
            plan,
            PathBuf::from("RPG/Alicesoft/Sample Game/Sample Game v1.02.zip")
        );
    }

    #[test]
    fn sanitizes_path_separators() {
        let plan = build_target_path("RPG", "Circle/Name", "Game:Name", None, "rar");
        assert_eq!(
            plan,
            PathBuf::from("RPG/Circle_Name/Game_Name/Game_Name.rar")
        );
    }
}
