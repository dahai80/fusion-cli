use crate::service::health::ServiceStatus;

use super::service_fetcher::{DashboardData, SystemInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Services,
    Models,
    System,
    Logs,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Services, Tab::Models, Tab::System, Tab::Logs]
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Services => "Services",
            Tab::Models => "Models",
            Tab::System => "System",
            Tab::Logs => "Logs",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Services => 0,
            Tab::Models => 1,
            Tab::System => 2,
            Tab::Logs => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Tab::Services,
            1 => Tab::Models,
            2 => Tab::System,
            _ => Tab::Logs,
        }
    }
}

pub struct App {
    pub tab: Tab,
    pub services: Vec<ServiceStatus>,
    pub models: Vec<String>,
    pub system: SystemInfo,
    pub logs: Vec<String>,
    pub running: bool,
    pub selected: usize,
    pub last_refresh: String,
}

impl App {
    pub fn new() -> Self {
        let data = DashboardData::empty();
        Self {
            tab: Tab::Services,
            services: data.services,
            models: data.models,
            system: data.system,
            logs: data.logs,
            running: true,
            selected: 0,
            last_refresh: chrono::Local::now().format("%H:%M:%S").to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn tick(&mut self) {}

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next_tab(&mut self) {
        let idx = self.tab.index();
        self.tab = Tab::from_index((idx + 1) % Tab::all().len());
        self.selected = 0;
    }

    pub fn prev_tab(&mut self) {
        let idx = self.tab.index();
        let total = Tab::all().len();
        self.tab = Tab::from_index((idx + total - 1) % total);
        self.selected = 0;
    }

    pub fn down(&mut self) {
        let max = self.list_len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn update_data(&mut self, data: DashboardData) {
        self.services = data.services;
        self.models = data.models;
        self.system = data.system;
        self.logs = data.logs;
        self.last_refresh = chrono::Local::now().format("%H:%M:%S").to_string();
        let max = self.list_len();
        if self.selected >= max && max > 0 {
            self.selected = max - 1;
        }
    }

    fn list_len(&self) -> usize {
        match self.tab {
            Tab::Services => self.services.len(),
            Tab::Models => self.models.len(),
            Tab::System => 4,
            Tab::Logs => self.logs.len(),
        }
    }

    pub fn selected_service(&self) -> Option<&ServiceStatus> {
        self.services.get(self.selected)
    }

    pub fn services_up(&self) -> usize {
        self.services.iter().filter(|s| s.alive).count()
    }
}
