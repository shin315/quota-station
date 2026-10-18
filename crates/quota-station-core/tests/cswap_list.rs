//! `cswap list --json`: every window of every account, its email and its
//! organization. Fixture names are made up.

mod common;

use common::{fixture_path, script, temp_dir};
use quota_station_core::cswap::{self, ListError, Usage};

const LIST: &str = include_str!("fixtures/cswap-list.json");

#[test]
fn every_window_of_every_account_is_read() {
    let list = cswap::parse_list(LIST).expect("the fixture reads");
    assert_eq!(list.len(), 3);
    let personal = &list[0];
    assert_eq!(personal.number, 1);
    assert_eq!(personal.email.as_deref(), Some("lanpham.dev@example.com"));
    // A five-hour window that has not started: used 0, no reset time.
    assert_eq!(personal.five_hour, Some(Usage { used: 0.0, resets_at: None }));
    assert_eq!(personal.seven_day.as_ref().map(|u| u.used), Some(2.0));
    assert_eq!(personal.fable.as_ref().map(|u| u.used), Some(0.0));
    let team = &list[1];
    assert_eq!(team.organization.as_deref(), Some("Acme Studio"));
    assert_eq!(team.five_hour.as_ref().map(|u| u.used), Some(38.0));
    assert_eq!(
        team.five_hour.as_ref().and_then(|u| u.resets_at.as_deref()),
        Some("2030-01-15T16:40:00.397549+00:00")
    );
    // Team seats report no weekly window.
    assert_eq!(team.seven_day, None);
    assert_eq!(team.fable.as_ref().map(|u| u.used), Some(11.0));
    assert_eq!(list[2].five_hour.as_ref().map(|u| u.used), Some(90.0));
}

#[test]
fn a_list_that_is_not_json_reads_as_none() {
    assert_eq!(cswap::parse_list("No accounts managed"), None);
    assert_eq!(cswap::parse_list(""), None);
}

#[test]
fn missing_usage_and_empty_names_read_as_none() {
    let list =
        cswap::parse_list(r#"{"accounts":[{"number":4,"email":"","organizationName":" "}]}"#)
            .unwrap();
    assert_eq!(list[0].number, 4);
    assert_eq!(list[0].email, None);
    assert_eq!(list[0].organization, None);
    assert_eq!(list[0].five_hour, None);
}

#[test]
fn used_share_stays_within_0_and_100() {
    let list =
        cswap::parse_list(r#"{"accounts":[{"number":1,"usage":{"fiveHour":{"pct":104.5}}}]}"#)
            .unwrap();
    assert_eq!(list[0].five_hour.as_ref().map(|u| u.used), Some(100.0));
}

#[test]
fn list_asks_for_json() {
    assert_eq!(cswap::list_args(), vec!["list".to_string(), "--json".to_string()]);
}

#[test]
fn list_runs_a_fake_cswap() {
    let dir = temp_dir("cswap-list");
    let fake = script(
        &dir.join("cswap"),
        &format!(
            "[ \"$1 $2\" = \"list --json\" ] || exit 9\ncat '{}'\n",
            fixture_path("cswap-list.json").display()
        ),
    );
    let list = cswap::list(&fake).expect("the fake answers");
    assert_eq!(list.iter().map(|a| a.number).collect::<Vec<_>>(), vec![1, 2, 3]);
    let broken = script(&dir.join("broken"), "echo 'not json'\nexit 1\n");
    assert_eq!(cswap::list(&broken), Err(ListError::Unreadable(Some(1))));
    assert_eq!(cswap::list(&dir.join("missing")), Err(ListError::NotFound));
}
