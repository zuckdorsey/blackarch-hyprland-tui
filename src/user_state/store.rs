use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    error::{AppError, Result},
    models::{RecentTool, UserState},
    user_state::paths,
    utils::validate::{validate_executable_name, validate_package_name},
};

pub fn load_user_state() -> Result<UserState> {
    Ok(UserState {
        favorites: load_favorites()?,
        recent: load_recent()?,
    })
}

#[allow(dead_code)]
pub fn save_user_state(state: &UserState) -> Result<()> {
    save_favorites(&state.favorites)?;
    save_recent(&state.recent)
}

pub fn load_favorites() -> Result<Vec<String>> {
    load_favorites_from_path(&paths::favorites_path()?)
}

pub fn save_favorites(favorites: &[String]) -> Result<()> {
    paths::ensure_data_dir()?;
    save_favorites_to_path(&paths::favorites_path()?, favorites)
}

pub fn load_recent() -> Result<Vec<RecentTool>> {
    load_recent_from_path(&paths::recent_path()?)
}

#[allow(dead_code)]
pub fn save_recent(recent: &[RecentTool]) -> Result<()> {
    paths::ensure_data_dir()?;
    save_recent_to_path(&paths::recent_path()?, recent)
}

pub fn toggle_favorite(package_name: &str) -> Result<bool> {
    validate_user_package_name(package_name)?;
    let mut favorites = load_favorites()?;
    let added = toggle_favorite_in_vec(&mut favorites, package_name);
    save_favorites(&favorites)?;
    Ok(added)
}

pub fn is_favorite(package_name: &str) -> Result<bool> {
    validate_user_package_name(package_name)?;
    Ok(load_favorites()?
        .iter()
        .any(|favorite| favorite == package_name))
}

pub fn add_recent_tool(package_name: &str, executable: Option<&str>) -> Result<()> {
    validate_user_package_name(package_name)?;
    if let Some(executable) = executable {
        validate_user_executable_name(executable)?;
    }

    let mut recent = load_recent()?;
    add_recent_tool_to_vec(&mut recent, package_name, executable, now_timestamp());
    save_recent(&recent)
}

fn load_favorites_from_path(path: &Path) -> Result<Vec<String>> {
    let favorites = load_json::<Vec<String>>(path)?.unwrap_or_default();
    for favorite in &favorites {
        validate_user_package_name(favorite)?;
    }
    Ok(dedup(favorites))
}

fn save_favorites_to_path(path: &Path, favorites: &[String]) -> Result<()> {
    for favorite in favorites {
        validate_user_package_name(favorite)?;
    }
    save_json(path, &dedup(favorites.to_vec()))
}

fn load_recent_from_path(path: &Path) -> Result<Vec<RecentTool>> {
    let recent = load_json::<Vec<RecentTool>>(path)?.unwrap_or_default();
    for item in &recent {
        validate_user_package_name(&item.package_name)?;
        if let Some(executable) = &item.executable {
            validate_user_executable_name(executable)?;
        }
    }
    Ok(recent)
}

#[allow(dead_code)]
fn save_recent_to_path(path: &Path, recent: &[RecentTool]) -> Result<()> {
    for item in recent {
        validate_user_package_name(&item.package_name)?;
        if let Some(executable) = &item.executable {
            validate_user_executable_name(executable)?;
        }
    }
    save_json(path, recent)
}

fn add_recent_tool_to_vec(
    recent: &mut Vec<RecentTool>,
    package_name: &str,
    executable: Option<&str>,
    last_used: String,
) {
    recent.retain(|item| item.package_name != package_name);
    recent.insert(
        0,
        RecentTool {
            package_name: package_name.to_string(),
            executable: executable.map(ToString::to_string),
            last_used,
        },
    );
    recent.truncate(50);
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(AppError::Json)
}

fn save_json<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn toggle_favorite_in_vec(favorites: &mut Vec<String>, package_name: &str) -> bool {
    if let Some(index) = favorites
        .iter()
        .position(|favorite| favorite == package_name)
    {
        favorites.remove(index);
        false
    } else {
        favorites.push(package_name.to_string());
        favorites.sort();
        true
    }
}

fn dedup(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn validate_user_package_name(package_name: &str) -> Result<()> {
    if validate_package_name(package_name) {
        Ok(())
    } else {
        Err(AppError::InvalidPackageName(package_name.to_string()))
    }
}

fn validate_user_executable_name(executable: &str) -> Result<()> {
    if validate_executable_name(executable) {
        Ok(())
    } else {
        Err(AppError::InvalidPackageName(executable.to_string()))
    }
}

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_files_load_empty_state() {
        let dir = test_dir("missing");
        let favorites = load_favorites_from_path(&dir.join("favorites.json")).unwrap();
        let recent = load_recent_from_path(&dir.join("recent.json")).unwrap();
        assert!(favorites.is_empty());
        assert!(recent.is_empty());
    }

    #[test]
    fn saving_and_loading_favorites_works() {
        let path = test_dir("save_load").join("favorites.json");
        save_favorites_to_path(&path, &["sqlmap".to_string(), "nmap".to_string()]).unwrap();
        assert_eq!(
            load_favorites_from_path(&path).unwrap(),
            vec!["nmap", "sqlmap"]
        );
    }

    #[test]
    fn toggle_favorite_adds_and_removes_without_duplicates() {
        let mut favorites = Vec::new();
        assert!(toggle_favorite_in_vec(&mut favorites, "sqlmap"));
        assert!(!toggle_favorite_in_vec(&mut favorites, "sqlmap"));
        assert!(favorites.is_empty());

        assert!(toggle_favorite_in_vec(&mut favorites, "sqlmap"));
        assert!(toggle_favorite_in_vec(&mut favorites, "nmap"));
        assert_eq!(dedup(favorites), vec!["nmap", "sqlmap"]);
    }

    #[test]
    fn invalid_package_name_is_rejected() {
        let path = test_dir("invalid").join("favorites.json");
        let error = save_favorites_to_path(&path, &["bad/name".to_string()]).unwrap_err();
        assert!(matches!(error, AppError::InvalidPackageName(_)));
    }

    #[test]
    fn add_recent_tool_adds_entry() {
        let mut recent = Vec::new();
        add_recent_tool_to_vec(&mut recent, "sqlmap", Some("sqlmap"), "1".to_string());
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].package_name, "sqlmap");
        assert_eq!(recent[0].executable.as_deref(), Some("sqlmap"));
    }

    #[test]
    fn add_recent_tool_moves_existing_entry_to_top() {
        let mut recent = vec![RecentTool {
            package_name: "nmap".to_string(),
            executable: Some("nmap".to_string()),
            last_used: "1".to_string(),
        }];
        add_recent_tool_to_vec(&mut recent, "sqlmap", Some("sqlmap"), "2".to_string());
        add_recent_tool_to_vec(&mut recent, "nmap", Some("nmap"), "3".to_string());

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].package_name, "nmap");
        assert_eq!(recent[0].last_used, "3");
    }

    #[test]
    fn recent_list_is_capped_at_50() {
        let mut recent = Vec::new();
        for index in 0..60 {
            add_recent_tool_to_vec(
                &mut recent,
                &format!("tool-{index}"),
                None,
                index.to_string(),
            );
        }

        assert_eq!(recent.len(), 50);
        assert_eq!(recent[0].package_name, "tool-59");
    }

    #[test]
    fn invalid_recent_executable_is_rejected() {
        let path = test_dir("bad-exec").join("recent.json");
        let recent = vec![RecentTool {
            package_name: "sqlmap".to_string(),
            executable: Some("bad/exec".to_string()),
            last_used: "1".to_string(),
        }];

        let error = save_recent_to_path(&path, &recent).unwrap_err();
        assert!(matches!(error, AppError::InvalidPackageName(_)));
    }

    fn test_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("blackarch-hypr-tui-{label}-{nanos}"))
    }
}
