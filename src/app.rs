use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    config::settings,
    error::Result,
    event,
    models::{BlackArchTool, ToolStatus},
    services::tool_service,
    ui,
};

const VIRTUAL_CATEGORIES: [&str; 4] = ["All Tools", "Installed", "Favorites", "Recent"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Categories,
    Tools,
    Details,
    Search,
}

pub struct App {
    pub tools: Vec<BlackArchTool>,
    pub filtered_tools: Vec<BlackArchTool>,
    pub categories: Vec<String>,
    pub selected_tool_index: usize,
    pub selected_category_index: usize,
    pub search_query: String,
    pub focus: FocusPane,
    pub loading: bool,
    pub error_message: Option<String>,
    pub status_message: String,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            filtered_tools: Vec::new(),
            categories: VIRTUAL_CATEGORIES
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            selected_tool_index: 0,
            selected_category_index: 0,
            search_query: String::new(),
            focus: FocusPane::Tools,
            loading: false,
            error_message: None,
            status_message: "Ready".to_string(),
            should_quit: false,
        }
    }

    pub fn load_initial_data(&mut self) -> Result<()> {
        self.loading = true;
        self.status_message = "Loading BlackArch tools...".to_string();

        let config = settings::load_config()?;
        let prefer_cache = config.pacman.prefer_cache;
        let tools = tool_service::load_tools(prefer_cache)?;
        let categories = match tool_service::load_categories(prefer_cache) {
            Ok(categories) => categories,
            Err(_) if !tools.is_empty() => categories_from_tools(&tools),
            Err(error) => return Err(error),
        };

        self.set_backend_data(categories, tools);
        self.load_selected_tool_detail();
        self.status_message = format!("Ready • {} tools loaded", self.tools.len());
        self.error_message = None;
        self.loading = false;
        Ok(())
    }

    pub fn refresh_from_backend(&mut self) -> Result<()> {
        self.loading = true;
        self.status_message = "Syncing BlackArch tools...".to_string();

        let categories = tool_service::refresh_categories_cache()?;
        let tools = tool_service::refresh_tools_cache()?;

        self.set_backend_data(categories, tools);
        self.load_selected_tool_detail();
        self.status_message = "Cache synced successfully".to_string();
        self.error_message = None;
        self.loading = false;
        Ok(())
    }

    pub fn apply_filters(&mut self) {
        let selected_package = self.selected_tool().map(|tool| tool.package_name.clone());
        let selected_category = self
            .categories
            .get(self.selected_category_index)
            .map(String::as_str)
            .unwrap_or("All Tools");
        let query = self.search_query.to_lowercase();

        self.filtered_tools = self
            .tools
            .iter()
            .filter(|tool| matches_category(tool, selected_category))
            .filter(|tool| matches_search(tool, &query))
            .cloned()
            .collect();

        if let Some(package_name) = selected_package {
            if let Some(index) = self
                .filtered_tools
                .iter()
                .position(|tool| tool.package_name == package_name)
            {
                self.selected_tool_index = index;
                return;
            }
        }

        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
        } else {
            self.selected_tool_index = self
                .selected_tool_index
                .min(self.filtered_tools.len().saturating_sub(1));
        }
    }

    pub fn selected_tool(&self) -> Option<&BlackArchTool> {
        self.filtered_tools.get(self.selected_tool_index)
    }

    pub fn select_next_tool(&mut self) {
        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
            return;
        }

        self.selected_tool_index = (self.selected_tool_index + 1) % self.filtered_tools.len();
        self.load_selected_tool_detail();
    }

    pub fn select_previous_tool(&mut self) {
        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
            return;
        }

        self.selected_tool_index = if self.selected_tool_index == 0 {
            self.filtered_tools.len() - 1
        } else {
            self.selected_tool_index - 1
        };
        self.load_selected_tool_detail();
    }

    pub fn select_next_category(&mut self) {
        if self.categories.is_empty() {
            self.selected_category_index = 0;
            return;
        }

        self.selected_category_index = (self.selected_category_index + 1) % self.categories.len();
        self.apply_filters();
        self.load_selected_tool_detail();
    }

    pub fn select_previous_category(&mut self) {
        if self.categories.is_empty() {
            self.selected_category_index = 0;
            return;
        }

        self.selected_category_index = if self.selected_category_index == 0 {
            self.categories.len() - 1
        } else {
            self.selected_category_index - 1
        };
        self.apply_filters();
        self.load_selected_tool_detail();
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.apply_filters();
        self.load_selected_tool_detail();
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusPane::Categories => FocusPane::Tools,
            FocusPane::Tools => FocusPane::Details,
            FocusPane::Details | FocusPane::Search => FocusPane::Categories,
        };
    }

    pub fn enter_search(&mut self) {
        self.focus = FocusPane::Search;
        self.status_message = "Search mode".to_string();
    }

    pub fn exit_search(&mut self) {
        if self.focus == FocusPane::Search {
            self.focus = FocusPane::Tools;
            self.status_message = "Ready".to_string();
        }
    }

    pub fn push_search_char(&mut self, ch: char) {
        let mut query = self.search_query.clone();
        query.push(ch);
        self.set_search_query(query);
    }

    pub fn pop_search_char(&mut self) {
        let mut query = self.search_query.clone();
        query.pop();
        self.set_search_query(query);
    }

    pub fn toggle_selected_favorite(&mut self) {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No tool selected".to_string();
            return;
        };

        if let Some(tool) = self
            .tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            tool.favorite = !tool.favorite;
            self.status_message = if tool.favorite {
                format!("Added {} to favorites", tool.name)
            } else {
                format!("Removed {} from favorites", tool.name)
            };
        }

        self.apply_filters();
    }

    pub fn load_selected_tool_detail(&mut self) {
        let Some(selected) = self.selected_tool().cloned() else {
            return;
        };

        if selected.version.is_some()
            || selected.description.as_deref() == Some("Package info unavailable")
        {
            return;
        }

        let detail =
            tool_service::get_tool_detail_or_partial(&selected.package_name, Some(&selected));
        let package_name = detail.package_name.clone();

        if let Some(tool) = self
            .tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            *tool = detail.clone();
        }

        if let Some(tool) = self
            .filtered_tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            *tool = detail;
        }
    }

    pub fn set_error(&mut self, error: impl ToString) {
        self.loading = false;
        self.error_message = Some(error.to_string());
        self.status_message = "Backend error".to_string();
    }

    fn set_backend_data(&mut self, backend_categories: Vec<String>, tools: Vec<BlackArchTool>) {
        self.categories = VIRTUAL_CATEGORIES
            .into_iter()
            .map(ToString::to_string)
            .chain(backend_categories)
            .collect();
        self.tools = tools;
        self.selected_category_index = self
            .selected_category_index
            .min(self.categories.len().saturating_sub(1));
        self.apply_filters();
    }
}

pub fn run() -> Result<()> {
    let mut app = App::new();
    app.loading = true;
    app.status_message = "Loading BlackArch tools...".to_string();

    let mut terminal = init_terminal()?;
    terminal.draw(|frame| ui::render(frame, &app))?;

    if let Err(error) = app.load_initial_data() {
        app.set_error(error);
    }

    let result = run_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

pub fn display_category_name(category: &str) -> String {
    let category = category.strip_prefix("blackarch-").unwrap_or(category);
    category
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_category_filter(category: &str) -> String {
    if category.starts_with("blackarch-") || VIRTUAL_CATEGORIES.contains(&category) {
        category.to_string()
    } else {
        format!("blackarch-{}", category.to_lowercase().replace(' ', "-"))
    }
}

fn matches_category(tool: &BlackArchTool, selected_category: &str) -> bool {
    match selected_category {
        "All Tools" => true,
        "Installed" => tool.status == ToolStatus::Installed,
        "Favorites" => tool.favorite,
        "Recent" => false,
        category => {
            let normalized = normalize_category_filter(category);
            tool.category.as_deref() == Some(normalized.as_str())
                || tool
                    .categories
                    .iter()
                    .any(|category| category == &normalized)
        }
    }
}

fn matches_search(tool: &BlackArchTool, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    tool.name.to_lowercase().contains(query)
        || tool.package_name.to_lowercase().contains(query)
        || tool
            .category
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || tool
            .categories
            .iter()
            .any(|category| category.to_lowercase().contains(query))
        || tool
            .description
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
}

fn categories_from_tools(tools: &[BlackArchTool]) -> Vec<String> {
    let mut categories = Vec::new();
    for tool in tools {
        for category in &tool.categories {
            if !categories.iter().any(|item| item == category) {
                categories.push(category.clone());
            }
        }

        if let Some(category) = &tool.category {
            if !categories.iter().any(|item| item == category) {
                categories.push(category.clone());
            }
        }
    }
    categories.sort();
    categories
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        event::handle_events(app)?;
    }

    Ok(())
}
