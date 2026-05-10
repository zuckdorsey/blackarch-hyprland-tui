use crate::models::{PackageInfo, PackageSearchResult};

pub fn parse_blackarch_categories(output: &str) -> Vec<String> {
    let mut categories = Vec::new();

    for line in output.lines() {
        let Some(group) = line.split_whitespace().next() else {
            continue;
        };

        if group.starts_with("blackarch-") && !categories.iter().any(|item| item == group) {
            categories.push(group.to_string());
        }
    }

    categories.sort();
    categories
}

pub fn parse_tools_from_group(output: &str, category: &str) -> Vec<String> {
    let mut tools = Vec::new();

    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(group) = parts.next() else {
            continue;
        };
        let Some(tool) = parts.next() else {
            continue;
        };

        if group == category && !tools.iter().any(|item| item == tool) {
            tools.push(tool.to_string());
        }
    }

    tools
}

pub fn parse_all_available_blackarch_tools(output: &str) -> Vec<String> {
    let mut tools = Vec::new();

    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(category) = parts.next() else {
            continue;
        };
        let Some(tool) = parts.next() else {
            continue;
        };

        if category.starts_with("blackarch-") && !tools.iter().any(|item| item == tool) {
            tools.push(tool.to_string());
        }
    }

    tools.sort();
    tools
}

pub fn parse_all_blackarch_tool_categories(output: &str) -> Vec<(String, String)> {
    let mut tools = Vec::new();

    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(category) = parts.next() else {
            continue;
        };
        let Some(tool) = parts.next() else {
            continue;
        };

        if category.starts_with("blackarch-") {
            tools.push((tool.to_string(), category.to_string()));
        }
    }

    tools
}

pub fn parse_search_results(output: &str) -> Vec<PackageSearchResult> {
    let mut results = Vec::new();
    let mut current: Option<PackageSearchResult> = None;

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if line.starts_with(char::is_whitespace) {
            if let Some(result) = current.as_mut() {
                let description = line.trim();
                if !description.is_empty() {
                    result.description = Some(description.to_string());
                }
            }
            continue;
        }

        if let Some(result) = current.take() {
            if is_blackarch_search_result(&result) {
                results.push(result);
            }
        }

        current = parse_search_header(line);
    }

    if let Some(result) = current {
        if is_blackarch_search_result(&result) {
            results.push(result);
        }
    }

    results
}

pub fn parse_package_info(output: &str) -> PackageInfo {
    let mut info = PackageInfo {
        repository: None,
        name: String::new(),
        version: None,
        description: None,
        url: None,
        groups: Vec::new(),
        licenses: Vec::new(),
        installed: false,
        executables: Vec::new(),
    };

    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();

        match key.trim() {
            "Repository" => info.repository = non_empty(value),
            "Name" => info.name = value.to_string(),
            "Version" => info.version = non_empty(value),
            "Description" => info.description = non_empty(value),
            "URL" => info.url = non_empty(value),
            "Groups" => {
                info.groups = value
                    .split_whitespace()
                    .filter(|group| *group != "None")
                    .map(ToString::to_string)
                    .collect();
            }
            "Licenses" => {
                info.licenses = value
                    .split_whitespace()
                    .filter(|license| *license != "None")
                    .map(ToString::to_string)
                    .collect();
            }
            _ => {}
        }
    }

    info
}

pub fn parse_executables(output: &str) -> Vec<String> {
    let mut executables = Vec::new();

    for line in output.lines() {
        let Some(path) = line.split_whitespace().nth(1) else {
            continue;
        };

        let Some(executable) = path.strip_prefix("/usr/bin/") else {
            continue;
        };

        if executable.is_empty() || executable.contains('/') {
            continue;
        }

        if !executables.iter().any(|item| item == executable) {
            executables.push(executable.to_string());
        }
    }

    executables
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty() && value != "None").then(|| value.to_string())
}

fn parse_search_header(line: &str) -> Option<PackageSearchResult> {
    let mut parts = line.split_whitespace();
    let package = parts.next()?;
    let version = parts.next().and_then(non_empty);
    let (repository, name) = package.split_once('/')?;
    let remainder = parts.collect::<Vec<_>>().join(" ");
    let groups = remainder
        .split('[')
        .skip(1)
        .filter_map(|part| part.split_once(']').map(|(groups, _)| groups))
        .flat_map(|groups| groups.split_whitespace())
        .map(ToString::to_string)
        .collect();

    Some(PackageSearchResult {
        repository: repository.to_string(),
        name: name.to_string(),
        version,
        groups,
        description: None,
    })
}

fn is_blackarch_search_result(result: &PackageSearchResult) -> bool {
    result.repository == "blackarch"
        || result
            .groups
            .iter()
            .any(|group| group.starts_with("blackarch-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SG_SAMPLE: &str = "blackarch-webapp sqlmap\nblackarch-webapp nikto\nblackarch-scanner nmap\nblackarch-recon amass\n";

    const CATEGORY_SAMPLE: &str =
        "blackarch-webapp sqlmap\nblackarch-webapp nikto\nblackarch-webapp gobuster\n";

    const SI_SAMPLE: &str = "Repository      : blackarch\nName            : sqlmap\nVersion         : 1.7.11-1\nDescription     : Automatic SQL injection and database takeover tool\nArchitecture    : any\nURL             : https://sqlmap.org\nGroups          : blackarch-webapp\nLicenses        : GPL\n";

    const QL_SAMPLE: &str =
        "sqlmap /usr/\nsqlmap /usr/bin/\nsqlmap /usr/bin/sqlmap\nsqlmap /usr/share/\n";

    #[test]
    fn parses_categories() {
        assert_eq!(
            parse_blackarch_categories(SG_SAMPLE),
            vec!["blackarch-recon", "blackarch-scanner", "blackarch-webapp"]
        );
    }

    #[test]
    fn parses_tools_from_group() {
        assert_eq!(
            parse_tools_from_group(CATEGORY_SAMPLE, "blackarch-webapp"),
            vec!["sqlmap", "nikto", "gobuster"]
        );
    }

    #[test]
    fn parses_all_blackarch_tools() {
        assert_eq!(
            parse_all_available_blackarch_tools(&format!("{SG_SAMPLE}core pacman\n")),
            vec!["amass", "nikto", "nmap", "sqlmap"]
        );
    }

    #[test]
    fn parses_all_blackarch_tool_categories() {
        assert_eq!(
            parse_all_blackarch_tool_categories(SG_SAMPLE),
            vec![
                ("sqlmap".to_string(), "blackarch-webapp".to_string()),
                ("nikto".to_string(), "blackarch-webapp".to_string()),
                ("nmap".to_string(), "blackarch-scanner".to_string()),
                ("amass".to_string(), "blackarch-recon".to_string())
            ]
        );
    }

    #[test]
    fn parses_search_result() {
        let results = parse_search_results(
            "blackarch/sqlmap 1.7.11-1 [blackarch-webapp]\n    Automatic SQL injection and database takeover tool\n",
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].repository, "blackarch");
        assert_eq!(results[0].name, "sqlmap");
        assert_eq!(results[0].version.as_deref(), Some("1.7.11-1"));
        assert_eq!(results[0].groups, vec!["blackarch-webapp"]);
        assert_eq!(
            results[0].description.as_deref(),
            Some("Automatic SQL injection and database takeover tool")
        );
    }

    #[test]
    fn parses_package_info() {
        let info = parse_package_info(SI_SAMPLE);
        assert_eq!(info.name, "sqlmap");
        assert_eq!(info.repository.as_deref(), Some("blackarch"));
        assert_eq!(info.version.as_deref(), Some("1.7.11-1"));
        assert_eq!(
            info.description.as_deref(),
            Some("Automatic SQL injection and database takeover tool")
        );
        assert_eq!(info.url.as_deref(), Some("https://sqlmap.org"));
        assert_eq!(info.groups, vec!["blackarch-webapp"]);
        assert_eq!(info.licenses, vec!["GPL"]);
    }

    #[test]
    fn parses_executables() {
        assert_eq!(parse_executables(QL_SAMPLE), vec!["sqlmap"]);
    }
}
