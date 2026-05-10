use crate::models::PackageInfo;

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

pub fn parse_package_info(output: &str) -> PackageInfo {
    let mut info = PackageInfo {
        name: String::new(),
        version: None,
        description: None,
        url: None,
        groups: Vec::new(),
        installed: false,
        executables: Vec::new(),
    };

    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();

        match key.trim() {
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

#[cfg(test)]
mod tests {
    use super::*;

    const SG_SAMPLE: &str = "blackarch-webapp sqlmap\nblackarch-webapp nikto\nblackarch-scanner nmap\nblackarch-recon amass\n";

    const SI_SAMPLE: &str = "Repository      : blackarch\nName            : sqlmap\nVersion         : 1.7.11-1\nDescription     : Automatic SQL injection and database takeover tool\nArchitecture    : any\nURL             : https://sqlmap.org\nGroups          : blackarch-webapp\nLicenses        : GPL\n";

    const QL_SAMPLE: &str =
        "sqlmap /usr/\nsqlmap /usr/bin/\nsqlmap /usr/bin/sqlmap\nsqlmap /usr/share/\n";

    #[test]
    fn parses_categories() {
        assert_eq!(
            parse_blackarch_categories(SG_SAMPLE),
            vec!["blackarch-webapp", "blackarch-scanner", "blackarch-recon"]
        );
    }

    #[test]
    fn parses_tools_from_group() {
        assert_eq!(
            parse_tools_from_group(SG_SAMPLE, "blackarch-webapp"),
            vec!["sqlmap", "nikto"]
        );
    }

    #[test]
    fn parses_package_info() {
        let info = parse_package_info(SI_SAMPLE);
        assert_eq!(info.name, "sqlmap");
        assert_eq!(info.version.as_deref(), Some("1.7.11-1"));
        assert_eq!(
            info.description.as_deref(),
            Some("Automatic SQL injection and database takeover tool")
        );
        assert_eq!(info.url.as_deref(), Some("https://sqlmap.org"));
        assert_eq!(info.groups, vec!["blackarch-webapp"]);
    }

    #[test]
    fn parses_executables() {
        assert_eq!(parse_executables(QL_SAMPLE), vec!["sqlmap"]);
    }
}
