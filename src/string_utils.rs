//! String utility functions for common string operations.

/// Strip the first matching prefix from a string.
///
/// Iterates through the provided prefixes and returns the string with
/// the first matching prefix removed. If no prefix matches, returns the
/// original string unchanged.
///
/// # Arguments
///
/// * `s` - The string to process
/// * `prefixes` - A slice of string prefixes to try stripping
///
/// # Returns
///
/// The string with the first matching prefix removed, or the original
/// string if no prefix matched.
///
/// # Example
///
/// ```
/// use fruit::string_utils::strip_any_prefix;
///
/// const STD_PREFIXES: &[&str] = &["std::", "core::", "alloc::"];
/// assert_eq!(strip_any_prefix("std::path::Path", STD_PREFIXES), "path::Path");
/// assert_eq!(strip_any_prefix("core::option::Option", STD_PREFIXES), "option::Option");
/// assert_eq!(strip_any_prefix("other::module", STD_PREFIXES), "other::module");
/// ```
pub fn strip_any_prefix<'a>(s: &'a str, prefixes: &[&str]) -> &'a str {
    for prefix in prefixes {
        if let Some(stripped) = s.strip_prefix(prefix) {
            return stripped;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_any_prefix_matches_first() {
        const PREFIXES: &[&str] = &["std::", "core::", "alloc::"];
        assert_eq!(strip_any_prefix("std::path::Path", PREFIXES), "path::Path");
    }

    #[test]
    fn test_strip_any_prefix_matches_middle() {
        const PREFIXES: &[&str] = &["std::", "core::", "alloc::"];
        assert_eq!(
            strip_any_prefix("core::option::Option", PREFIXES),
            "option::Option"
        );
    }

    #[test]
    fn test_strip_any_prefix_matches_last() {
        const PREFIXES: &[&str] = &["std::", "core::", "alloc::"];
        assert_eq!(
            strip_any_prefix("alloc::vec::Vec", PREFIXES),
            "vec::Vec"
        );
    }

    #[test]
    fn test_strip_any_prefix_no_match() {
        const PREFIXES: &[&str] = &["std::", "core::", "alloc::"];
        assert_eq!(
            strip_any_prefix("other::module", PREFIXES),
            "other::module"
        );
    }

    #[test]
    fn test_strip_any_prefix_empty_prefixes() {
        const PREFIXES: &[&str] = &[];
        assert_eq!(strip_any_prefix("std::path::Path", PREFIXES), "std::path::Path");
    }

    #[test]
    fn test_strip_any_prefix_php_tags() {
        const PHP_PREFIXES: &[&str] = &["<?php", "<?"];
        assert_eq!(strip_any_prefix("<?php echo 'hi';", PHP_PREFIXES), " echo 'hi';");
        assert_eq!(strip_any_prefix("<? echo 'hi';", PHP_PREFIXES), " echo 'hi';");
        assert_eq!(strip_any_prefix("echo 'hi';", PHP_PREFIXES), "echo 'hi';");
    }

    #[test]
    fn test_strip_any_prefix_order_matters() {
        // Longer prefixes should come first to match correctly
        const PREFIXES: &[&str] = &["<?php", "<?"];
        assert_eq!(strip_any_prefix("<?php", PREFIXES), "");
        // If order is reversed, "<?php" would match "<?" and leave "php"
    }
}
