pub fn validate_package_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(is_safe_package_char)
}

pub fn validate_category_name(name: &str) -> bool {
    name.strip_prefix("blackarch-")
        .is_some_and(validate_package_name)
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
}
