//! Link picker methods on `App`.

use super::types::Overlay;
use super::App;

impl App {
    /// Return indices of links matching the current link search query.
    ///
    /// Results are cached and only recomputed when the search query or
    /// the number of document links changes.
    pub(crate) fn filtered_link_indices(&mut self) -> &[usize] {
        if self.link_picker.search_query != self.link_picker.cached_query
            || self.document.links.len() != self.link_picker.cached_count
        {
            self.link_picker.cached_query = self.link_picker.search_query.clone();
            self.link_picker.cached_count = self.document.links.len();
            self.link_picker.cached_indices = if self.link_picker.search_query.is_empty() {
                (0..self.document.links.len()).collect()
            } else {
                let query = self.link_picker.search_query.to_lowercase();
                self.document
                    .links
                    .iter()
                    .enumerate()
                    .filter(|(_, link)| {
                        link.display_text.to_lowercase().contains(&query)
                            || link.url.to_lowercase().contains(&query)
                    })
                    .map(|(i, _)| i)
                    .collect()
            };
        }
        &self.link_picker.cached_indices
    }

    pub(super) fn open_selected_link(&mut self) {
        let selected = self.link_picker.selected;
        let link_idx = self.filtered_link_indices().get(selected).copied();
        if let Some(link_idx) = link_idx {
            if let Some(link) = self.document.links.get(link_idx) {
                let url = link.url.clone();
                self.overlay = Overlay::None;
                self.link_picker.search_query.clear();
                self.status_message = format!("Opening: {url}");
                std::thread::spawn(move || {
                    let _ = open::that(&url);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use crate::app::App;
    use crate::markdown::LinkInfo;
    use crate::test_util::TempTestDir;

    #[test]
    fn filtered_link_indices_empty_query_returns_all() {
        let dir = TempTestDir::new("mdt-test-link-all");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links = vec![
            LinkInfo { display_text: "Google".to_string(), url: "https://google.com".to_string() },
            LinkInfo { display_text: "GitHub".to_string(), url: "https://github.com".to_string() },
        ];
        app.link_picker.search_query.clear();

        let indices = app.filtered_link_indices().to_vec();
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn filtered_link_indices_filters_by_display_text() {
        let dir = TempTestDir::new("mdt-test-link-filter-text");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links = vec![
            LinkInfo { display_text: "Google".to_string(), url: "https://google.com".to_string() },
            LinkInfo { display_text: "GitHub".to_string(), url: "https://github.com".to_string() },
            LinkInfo { display_text: "Rust".to_string(), url: "https://rust-lang.org".to_string() },
        ];
        app.link_picker.search_query = "git".to_string();

        let indices = app.filtered_link_indices().to_vec();
        assert_eq!(indices, vec![1]); // Only GitHub matches
    }

    #[test]
    fn filtered_link_indices_filters_by_url() {
        let dir = TempTestDir::new("mdt-test-link-filter-url");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links = vec![
            LinkInfo { display_text: "Search".to_string(), url: "https://google.com".to_string() },
            LinkInfo { display_text: "Code".to_string(), url: "https://github.com".to_string() },
        ];
        app.link_picker.search_query = "google".to_string();

        let indices = app.filtered_link_indices().to_vec();
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn filtered_link_indices_caches_results() {
        let dir = TempTestDir::new("mdt-test-link-cache");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links =
            vec![LinkInfo { display_text: "A".to_string(), url: "https://a.com".to_string() }];
        app.link_picker.search_query = "a".to_string();

        // First call computes
        let indices1 = app.filtered_link_indices().to_vec();
        // Second call uses cache (same query, same link count)
        let indices2 = app.filtered_link_indices().to_vec();
        assert_eq!(indices1, indices2);
        assert_eq!(app.link_picker.cached_query, "a");
        assert_eq!(app.link_picker.cached_count, 1);
    }

    #[test]
    fn filtered_link_indices_cache_invalidated_on_query_change() {
        let dir = TempTestDir::new("mdt-test-link-cache-inv");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links = vec![
            LinkInfo { display_text: "Alpha".to_string(), url: "https://alpha.com".to_string() },
            LinkInfo { display_text: "Beta".to_string(), url: "https://beta.com".to_string() },
        ];

        app.link_picker.search_query = "alpha".to_string();
        let idx1 = app.filtered_link_indices().to_vec();
        assert_eq!(idx1, vec![0]);

        app.link_picker.search_query = "beta".to_string();
        let idx2 = app.filtered_link_indices().to_vec();
        assert_eq!(idx2, vec![1]);
    }

    #[test]
    fn filtered_link_indices_case_insensitive() {
        let dir = TempTestDir::new("mdt-test-link-case");
        dir.create_file("t.md", "# T");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.document.links = vec![LinkInfo {
            display_text: "GitHub".to_string(),
            url: "https://github.com".to_string(),
        }];
        app.link_picker.search_query = "GITHUB".to_string();

        let indices = app.filtered_link_indices().to_vec();
        assert_eq!(indices, vec![0]);
    }
}
