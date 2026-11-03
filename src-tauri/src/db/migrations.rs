use rusqlite_migration::Migrations;

/// Every schema change, oldest first. A released migration is never edited (DATA_MODEL §8).
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![])
}
