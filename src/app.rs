use crate::host::{Host, HostStatus};
use std::sync::{Arc, Mutex};

pub struct App {
    pub hosts: Arc<Mutex<Vec<Host>>>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub filter: String,
    pub filter_mode: bool,
    pub should_quit: bool,
    pub connect_index: Option<usize>,
    pub show_groups: bool,
    pub message: Option<String>,
}

impl App {
    pub fn new(hosts: Vec<Host>) -> Self {
        Self {
            hosts: Arc::new(Mutex::new(hosts)),
            selected: 0,
            scroll_offset: 0,
            filter: String::new(),
            filter_mode: false,
            should_quit: false,
            connect_index: None,
            show_groups: true,
            message: None,
        }
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let hosts = self.hosts.lock().unwrap();
        if self.filter.is_empty() {
            return (0..hosts.len()).collect();
        }
        let query = self.filter.to_lowercase();
        hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| {
                h.alias.to_lowercase().contains(&query)
                    || h.hostname.to_lowercase().contains(&query)
                    || h.group.to_lowercase().contains(&query)
                    || h.user.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn select_down(&mut self) {
        let max = self.filtered_indices().len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub fn page_up(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
    }

    pub fn page_down(&mut self, n: usize) {
        let max = self.filtered_indices().len().saturating_sub(1);
        self.selected = (self.selected + n).min(max);
    }

    pub fn connect_selected(&mut self) {
        let indices = self.filtered_indices();
        if let Some(&real_idx) = indices.get(self.selected) {
            self.connect_index = Some(real_idx);
        }
    }

    pub fn selected_host_index(&self) -> Option<usize> {
        let indices = self.filtered_indices();
        indices.get(self.selected).copied()
    }

    pub fn groups(&self) -> Vec<String> {
        let hosts = self.hosts.lock().unwrap();
        let mut groups: Vec<String> = hosts.iter().map(|h| h.group.clone()).collect();
        groups.sort();
        groups.dedup();
        groups
    }
}
