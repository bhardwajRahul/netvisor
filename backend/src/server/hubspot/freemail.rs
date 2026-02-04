//! Free and disposable email domain detection
//!
//! Data sourced from https://github.com/willwhite/freemail
//! - free.txt: ~4,500 free email provider domains (Gmail, Yahoo, etc.)
//! - disposable.txt: ~88,000 disposable/temporary email domains

use email_address::EmailAddress;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Free email domains (Gmail, Yahoo, etc.) from willwhite/freemail free.txt
static FREE_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("freemail_free.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
});

/// Disposable email domains from willwhite/freemail disposable.txt
static DISPOSABLE_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("freemail_disposable.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
});

/// Check if email is from a free provider (Gmail, Yahoo, etc.)
pub fn is_free_email(email: &EmailAddress) -> bool {
    FREE_DOMAINS.contains(email.domain().to_lowercase().as_str())
}

/// Check if email is from a disposable provider (Mailinator, etc.)
pub fn is_disposable_email(email: &EmailAddress) -> bool {
    DISPOSABLE_DOMAINS.contains(email.domain().to_lowercase().as_str())
}

/// Check if email is from a work/business domain (not free, not disposable)
pub fn is_work_email(email: &EmailAddress) -> bool {
    !is_free_email(email) && !is_disposable_email(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_email_detection() {
        // Major free providers should be detected
        assert!(is_free_email(&"user@gmail.com".parse().unwrap()));
        assert!(is_free_email(&"user@yahoo.com".parse().unwrap()));
        assert!(is_free_email(&"user@hotmail.com".parse().unwrap()));
        assert!(is_free_email(&"user@outlook.com".parse().unwrap()));

        // Case insensitive
        assert!(is_free_email(&"user@GMAIL.COM".parse().unwrap()));
        assert!(is_free_email(&"user@Gmail.Com".parse().unwrap()));

        // Business domains should not be detected as free
        assert!(!is_free_email(&"user@company.com".parse().unwrap()));
        assert!(!is_free_email(&"user@acme.io".parse().unwrap()));
    }

    #[test]
    fn test_disposable_email_detection() {
        // Known disposable providers should be detected
        assert!(is_disposable_email(&"user@mailinator.com".parse().unwrap()));
        assert!(is_disposable_email(
            &"user@guerrillamail.com".parse().unwrap()
        ));

        // Case insensitive
        assert!(is_disposable_email(&"user@MAILINATOR.COM".parse().unwrap()));

        // Business domains should not be detected as disposable
        assert!(!is_disposable_email(&"user@company.com".parse().unwrap()));

        // Free emails are not disposable (separate lists)
        assert!(!is_disposable_email(&"user@gmail.com".parse().unwrap()));
    }

    #[test]
    fn test_work_email_detection() {
        // Business domains should be detected as work email
        assert!(is_work_email(&"user@company.com".parse().unwrap()));
        assert!(is_work_email(&"user@acme.io".parse().unwrap()));
        assert!(is_work_email(&"user@scanopy.com".parse().unwrap()));

        // Free providers should not be detected as work email
        assert!(!is_work_email(&"user@gmail.com".parse().unwrap()));
        assert!(!is_work_email(&"user@yahoo.com".parse().unwrap()));

        // Disposable providers should not be detected as work email
        assert!(!is_work_email(&"user@mailinator.com".parse().unwrap()));
    }
}
