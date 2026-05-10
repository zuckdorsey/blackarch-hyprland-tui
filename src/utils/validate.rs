pub fn validate_package_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(is_safe_package_char)
}

pub fn validate_category_name(name: &str) -> bool {
    name.strip_prefix("blackarch-")
        .is_some_and(validate_package_name)
}

pub fn validate_executable_name(name: &str) -> bool {
    validate_package_name(name)
}

pub fn validate_terminal_program(program: &str) -> bool {
    matches!(program, "kitty" | "foot" | "alacritty" | "wezterm")
}

pub fn validate_terminal_class(class_name: &str) -> bool {
    !class_name.is_empty()
        && class_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn is_safe_package_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '+')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_package_names() {
        assert!(validate_package_name("aircrack-ng"));
        assert!(validate_package_name("libfoo_1.2+bar"));
    }

    #[test]
    fn rejects_unsafe_package_names() {
        assert!(!validate_package_name("sqlmap;rm"));
        assert!(!validate_package_name("sql map"));
        assert!(!validate_package_name("../sqlmap"));
        assert!(!validate_package_name("$sqlmap"));
        assert!(!validate_package_name("'sqlmap'"));
    }

    #[test]
    fn validates_blackarch_categories() {
        assert!(validate_category_name("blackarch-webapp"));
        assert!(!validate_category_name("webapp"));
        assert!(!validate_category_name("blackarch-webapp;bad"));
    }

    #[test]
    fn validates_executable_names() {
        assert!(validate_executable_name("sqlmap"));
        assert!(validate_executable_name("aircrack-ng"));
        assert!(!validate_executable_name("bin/sqlmap"));
        assert!(!validate_executable_name("sql map"));
        assert!(!validate_executable_name("sqlmap;rm"));
    }

    #[test]
    fn validates_terminal_programs() {
        assert!(validate_terminal_program("kitty"));
        assert!(validate_terminal_program("foot"));
        assert!(validate_terminal_program("alacritty"));
        assert!(validate_terminal_program("wezterm"));
        assert!(!validate_terminal_program("xterm"));
        assert!(!validate_terminal_program("kitty;bad"));
    }

    #[test]
    fn validates_terminal_classes() {
        assert!(validate_terminal_class("blackarch-tool-runner"));
        assert!(validate_terminal_class("runner.Class_1"));
        assert!(!validate_terminal_class("runner class"));
        assert!(!validate_terminal_class("runner/class"));
        assert!(!validate_terminal_class("runner;bad"));
    }
}
