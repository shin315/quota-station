// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The main window's tabs of the advanced part ([`Extension::tabs`]): the
//! rotation's rules and, on a Mac, the copy to another Mac, by the names
//! their pages have always had.
//!
//! [`Extension::tabs`]: crate::ext::Extension::tabs

/// The rules editor of the rotation (Xoay vòng), where `Thoát` asks about
/// unsaved rules ([`super::close`]).
pub const RULES: &str = "rules";
/// The copy to another Mac.
#[cfg(unix)]
const COPY: &str = "copy";

/// The tabs, in the order the main window had them.
#[cfg(unix)]
pub const TABS: &[&str] = &[RULES, COPY];
#[cfg(not(unix))]
pub const TABS: &[&str] = &[RULES];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advanced::Advanced;
    use crate::ext::Tab;
    use crate::ext::{shadowed_tabs, Extension, NoExtension};

    /// The names the private variants of `Tab` were written as (serde's
    /// camelCase of `Rules` and, on a Mac, `Copy`).
    fn names() -> Vec<&'static str> {
        let mut names = vec!["rules"];
        if cfg!(unix) {
            names.push("copy");
        }
        names
    }

    #[test]
    fn the_private_tabs_keep_their_names() {
        let mut listed = Advanced.tabs().to_vec();
        listed.sort_unstable();
        let mut names = names();
        names.sort_unstable();
        assert_eq!(listed, names);
        assert_eq!(shadowed_tabs(Advanced.tabs()), Vec::<&str>::new());
    }

    #[test]
    fn each_private_tab_name_reads_and_writes_back_unchanged() {
        for name in names() {
            let tab = Tab::named(name, Advanced.tabs()).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(tab, Tab::Ext(name.into()));
            assert_eq!(tab.name(), name);
            assert_eq!(serde_json::to_value(&tab).unwrap(), serde_json::json!(name));
        }
        assert_eq!(Tab::named(RULES, Advanced.tabs()), Some(Tab::Ext("rules".into())));
    }

    /// A build without the advanced part refuses its tab names, as it did
    /// when they were not variants of `Tab` there: nothing opens.
    #[test]
    fn without_the_extension_a_private_tab_name_opens_nothing() {
        for name in names() {
            assert_eq!(Tab::named(name, NoExtension.tabs()), None, "{name}");
        }
    }
}
