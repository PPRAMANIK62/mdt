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
