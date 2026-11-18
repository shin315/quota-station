// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! What a notification says of an account Google asks something of: a
//! verification, with or without the page for it, or a subscription. The
//! app's own words, never Google's message.

/// The title and the body of the card for an account in `needsVerification`:
/// `reason` is Google's reason code when it gave one, `page` whether it sent
/// the verification page.
pub fn verification(name: &str, reason: Option<&str>, page: bool) -> (String, String) {
    let again =
        "App tự đọc lại mỗi 30 phút, hoặc ngay khi bạn bấm Kiểm tra lại trong tab Antigravity.";
    match (reason, page) {
        (Some("SUBSCRIPTION_REQUIRED"), _) => (
            format!("{name} cần gói đăng ký"),
            format!("Google báo tài khoản này cần gói đăng ký trước khi cho đọc hạn mức; đăng nhập lại không đổi được điều đó. {again}"),
        ),
        (_, true) => (
            format!("{name} cần xác minh"),
            format!("Google cần xác minh tài khoản này trước khi cho đọc hạn mức. Xác minh xong, app đọc lại. {again}"),
        ),
        (_, false) => (
            format!("{name} cần xác minh"),
            format!("Google cần xác minh tài khoản này nhưng không gửi trang xác minh. Xác minh trong Antigravity hoặc ở tài khoản Google. {again}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_says_what_google_asked() {
        let (title, body) = verification("U2", Some("SUBSCRIPTION_REQUIRED"), false);
        assert_eq!(title, "U2 cần gói đăng ký");
        assert!(body.contains("đăng nhập lại không đổi được"));
        let (title, body) = verification("U2", Some("VALIDATION_REQUIRED"), true);
        assert_eq!(title, "U2 cần xác minh");
        assert!(!body.contains("không gửi trang"));
        let (_, body) = verification("U2", None, false);
        assert!(body.contains("không gửi trang xác minh"));
        assert!(body.contains("Kiểm tra lại"));
    }
}
