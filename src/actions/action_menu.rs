use crate::models::{ActionMenuItem, ActionMenuState};

pub fn default_state() -> ActionMenuState {
    ActionMenuState::default()
}

pub fn item_label(item: ActionMenuItem, favorite: bool) -> &'static str {
    match item {
        ActionMenuItem::RunInTerminal => "Run in terminal",
        ActionMenuItem::InstallOrUpdate => "Install / Update",
        ActionMenuItem::Remove => "Remove",
        ActionMenuItem::ToggleFavorite if favorite => "Remove from Favorites",
        ActionMenuItem::ToggleFavorite => "Add to Favorites",
        ActionMenuItem::CopyCommand => "Copy Command",
        ActionMenuItem::RefreshDetails => "Refresh Details",
        ActionMenuItem::PackageInfo => "Package Info",
        ActionMenuItem::Cancel => "Cancel",
    }
}
