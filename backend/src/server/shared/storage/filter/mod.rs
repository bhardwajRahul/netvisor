use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::base::DaemonMode,
    shared::{entities::EntityDiscriminants, storage::traits::SqlValue},
    users::r#impl::permissions::UserOrgPermissions,
};

use super::traits::Storable;

/// Builder pattern for common WHERE clauses with optional pagination and JOINs.
/// Generic over entity type T to automatically qualify column names with the table name.
#[derive(Clone)]
pub struct StorableFilter<T: Storable> {
    _marker: PhantomData<T>,
    conditions: Vec<String>,
    values: Vec<SqlValue>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
    joins: Vec<String>,
}

impl<T: Storable> Default for StorableFilter<T> {
    fn default() -> Self {
        Self::new()
    }
}

mod constructors;
mod filters;
mod lldp;
mod organization;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::Host;
    use crate::server::snapshots::types::base::Snapshot;
    use crate::server::tags::r#impl::base::Tag;
    use chrono::TimeZone;

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    #[test]
    fn id_or_lineage_in_emits_or_clause() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&ids);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.id = ANY($1)"),
            "expected id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.lineage_id = ANY($2)"),
            "expected lineage_id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains(" OR "),
            "expected OR-join in: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            2,
            "expected two array params (id_set, lineage_set)"
        );
    }

    #[test]
    fn id_or_lineage_in_empty_input_matches_nothing() {
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&[]);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("FALSE"),
            "empty id list should produce FALSE: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            0,
            "no params should be bound for empty input"
        );
    }

    #[test]
    fn taken_at_lt_emits_less_than_clause() {
        let cutoff = chrono::Utc::now();
        let filter = StorableFilter::<Snapshot>::new_unfiltered().taken_at_lt(cutoff);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("snapshots.taken_at < $1"),
            "expected `taken_at < $N` in: {}",
            where_clause
        );
        assert_eq!(filter.values().len(), 1);
    }

    #[test]
    fn live_filter_emits_valid_to_is_null() {
        let filter = StorableFilter::<Tag>::new_unfiltered().live();
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected `valid_to IS NULL` in: {}",
            where_clause
        );
    }

    #[test]
    fn as_of_filter_emits_window_predicate() {
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().as_of(t);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_none_is_live() {
        // `at = None` (live view) must hide closed historical copies, i.e. behave
        // exactly like `.live()`. This is the read-path guard that stops snapshot
        // close-and-clone copies leaking into entity lists as empty-shell dupes.
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(None);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected live predicate in: {}",
            where_clause
        );
        assert!(
            !where_clause.contains("valid_from <="),
            "live view must not emit an as-of window: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_some_is_as_of() {
        // `at = Some(t)` (snapshot view) must read SCD2 state as of `t`.
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(Some(t));
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected as-of lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected as-of upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn created_between_emits_inclusive_range() {
        let f = StorableFilter::<Host>::new().created_between(ts(100), ts(200));
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("created_at"), "condition was: {c}");
        assert!(
            c.contains(">= $1") && c.contains("<= $2"),
            "condition was: {c}"
        );
        assert_eq!(f.values.len(), 2);
        match (&f.values[0], &f.values[1]) {
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => {
                assert_eq!(*a, ts(100));
                assert_eq!(*b, ts(200));
            }
            _ => panic!("expected two Timestamp values"),
        }
    }

    #[test]
    fn updated_between_uses_updated_at_column() {
        let f = StorableFilter::<Host>::new().updated_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("updated_at"));
    }

    #[test]
    fn valid_to_between_uses_valid_to_column() {
        let f = StorableFilter::<Host>::new().valid_to_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("valid_to"));
    }

    #[test]
    fn last_seen_between_uses_last_seen_at_column() {
        let f = StorableFilter::<Host>::new().last_seen_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("last_seen_at"));
    }

    #[test]
    fn between_helpers_advance_param_index() {
        let f = StorableFilter::<Host>::new()
            .created_between(ts(10), ts(20))
            .updated_between(ts(30), ts(40));
        assert_eq!(f.values.len(), 4);
        assert!(f.conditions[0].contains("$1") && f.conditions[0].contains("$2"));
        assert!(f.conditions[1].contains("$3") && f.conditions[1].contains("$4"));
    }

    #[test]
    fn user_permissions_in_emits_in_clause() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[UserOrgPermissions::Owner, UserOrgPermissions::Admin]);
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("permissions"));
        assert!(c.contains("IN ($1, $2)"), "condition was: {c}");
        assert_eq!(f.values.len(), 2);
    }

    #[test]
    fn user_permissions_in_empty_emits_false() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[]);
        assert_eq!(f.conditions, vec!["FALSE".to_string()]);
        assert_eq!(f.values.len(), 0);
    }
}
