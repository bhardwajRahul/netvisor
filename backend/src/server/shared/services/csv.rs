//! CSV export utilities for entities.
//!
//! Provides service-layer CSV generation that can be used by handlers
//! and other services (e.g., for bundling multiple CSVs into a zip).

use crate::server::shared::storage::traits::Entity;

/// Build CSV bytes from a list of entities.
///
/// Headers are derived automatically from the CsvRow struct field names.
/// The Entity trait ensures `to_csv_row()` is implemented.
pub fn build_csv<T: Entity>(entities: &[T]) -> Result<Vec<u8>, csv::Error> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    for entity in entities {
        wtr.serialize(entity.to_csv_row())?;
    }

    wtr.into_inner()
        .map_err(|e| csv::Error::from(e.into_error()))
}
