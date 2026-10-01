//! Prefix search implementation

use crate::discovery::AppEntry;

/// Apps whose name starts with the query (case-insensitive), in their original order
pub fn prefix_search(apps: &[AppEntry], query: &str) -> Vec<AppEntry> {
    let query_lower = query.to_lowercase();

    apps.iter()
        .filter(|app| app.name.to_lowercase().starts_with(&query_lower))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps(names: &[&str]) -> Vec<AppEntry> {
        names
            .iter()
            .map(|name| AppEntry {
                name: name.to_string(),
                exec_path: None,
                icon_data: None,
            })
            .collect()
    }

    fn names(results: &[AppEntry]) -> Vec<&str> {
        results.iter().map(|app| app.name.as_str()).collect()
    }

    #[test]
    fn matches_only_names_starting_with_query() {
        let list = apps(&["Word", "WinDbg", "Notepad", "PowerShell", "Weather"]);

        assert_eq!(names(&prefix_search(&list, "w")), ["Word", "WinDbg", "Weather"]);
        assert_eq!(names(&prefix_search(&list, "we")), ["Weather"]);
        assert!(prefix_search(&list, "wex").is_empty());
    }

    #[test]
    fn ignores_case() {
        let list = apps(&["Visual Studio Code", "vim"]);

        assert_eq!(names(&prefix_search(&list, "V")), ["Visual Studio Code", "vim"]);
        assert_eq!(names(&prefix_search(&list, "VISUAL")), ["Visual Studio Code"]);
    }

    #[test]
    fn keeps_original_order() {
        let list = apps(&["Visual Studio Code", "Visio", "Calculator", "Visual Studio 2022"]);

        assert_eq!(
            names(&prefix_search(&list, "vis")),
            ["Visual Studio Code", "Visio", "Visual Studio 2022"]
        );
    }
}
