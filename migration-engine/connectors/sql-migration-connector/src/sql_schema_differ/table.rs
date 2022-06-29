use super::{differ_database::DifferDatabase, foreign_keys_match};
use crate::{flavour::SqlFlavour, pair::Pair};
use sql_schema_describer::{
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, SqlSchemaExt, TableWalker},
    PrimaryKey, TableId,
};

pub(crate) struct TableDiffer<'a, 'b> {
    pub(crate) tables: Pair<TableWalker<'a>>,
    pub(crate) db: &'b DifferDatabase<'a>,
}

impl<'schema, 'b> TableDiffer<'schema, 'b> {
    pub(crate) fn column_pairs(&self) -> impl Iterator<Item = Pair<ColumnWalker<'schema>>> + '_ {
        self.db
            .column_pairs(self.tables.map(|t| t.id))
            .map(move |colids| self.db.schemas().columns(colids))
    }

    pub(crate) fn any_column_changed(&self) -> bool {
        self.column_pairs()
            .any(|col| self.db.column_changes_for_walkers(col).differs_in_something())
    }

    pub(crate) fn dropped_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.db
            .dropped_columns(self.tables.map(|t| t.id))
            .map(move |colid| self.tables.previous.schema.walk_column(colid))
    }

    pub(crate) fn added_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.db
            .created_columns(self.tables.map(|t| t.id))
            .map(move |colid| self.tables.next.schema.walk_column(colid))
    }

    pub(crate) fn created_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next_foreign_keys().filter(move |next_fk| {
            !self
                .previous_foreign_keys()
                .any(|previous_fk| super::foreign_keys_match(Pair::new(&previous_fk, next_fk), self.db))
        })
    }

    pub(crate) fn dropped_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous_foreign_keys().filter(move |previous_fk| {
            !self
                .next_foreign_keys()
                .any(|next_fk| super::foreign_keys_match(Pair::new(previous_fk, &next_fk), self.db))
        })
    }

    pub(crate) fn created_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.next_indexes().filter(move |next_index| {
            !self
                .previous_indexes()
                .any(move |previous_index| indexes_match(previous_index, *next_index, self.db.flavour))
        })
    }

    pub(crate) fn dropped_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.previous_indexes().filter(move |previous_index| {
            !self
                .next_indexes()
                .any(|next_index| indexes_match(*previous_index, next_index, self.db.flavour))
        })
    }

    pub(crate) fn foreign_key_pairs(&self) -> impl Iterator<Item = Pair<ForeignKeyWalker<'schema>>> + '_ {
        self.previous_foreign_keys().filter_map(move |previous_fk| {
            self.next_foreign_keys()
                .find(move |next_fk| foreign_keys_match(Pair::new(&previous_fk, next_fk), self.db))
                .map(move |next_fk| Pair::new(previous_fk, next_fk))
        })
    }

    pub(crate) fn index_pairs<'a>(&'a self) -> impl Iterator<Item = Pair<IndexWalker<'schema>>> + 'a {
        let singular_indexes = self.previous_indexes().filter(move |left| {
            // Renaming an index in a situation where we have multiple indexes
            // with the same columns, but a different name, is highly unstable.
            // We do not rename them for now.
            let number_of_identical_indexes = self
                .previous_indexes()
                .filter(|right| {
                    left.column_names().len() == right.column_names().len()
                        && left.column_names().zip(right.column_names()).all(|(a, b)| a == b)
                        && left.index_type() == right.index_type()
                })
                .count();

            number_of_identical_indexes == 1
        });

        singular_indexes.filter_map(move |previous_index| {
            self.next_indexes()
                .find(|next_index| indexes_match(previous_index, *next_index, self.db.flavour))
                .map(|renamed_index| Pair::new(previous_index, renamed_index))
        })
    }

    pub(crate) fn primary_key_changed(&self) -> bool {
        match self.tables.as_ref().map(|t| t.primary_key()).into_tuple() {
            (Some(previous_pk), Some(next_pk)) => {
                if previous_pk.columns != next_pk.columns {
                    return true;
                }

                if self.primary_key_column_changed(previous_pk) {
                    return true;
                }

                self.db.flavour.primary_key_changed(self.tables)
            }
            _ => false,
        }
    }

    /// The primary key present in `next` but not `previous`, if applicable.
    pub(crate) fn created_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match self.tables.as_ref().map(|t| t.primary_key()).into_tuple() {
            (None, Some(pk)) => Some(pk),
            _ => None,
        }
    }

    /// The primary key present in `previous` but not `next`, if applicable.
    pub(crate) fn dropped_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match self.tables.as_ref().map(|t| t.primary_key()).into_tuple() {
            (Some(pk), None) => Some(pk),
            _ => None,
        }
    }

    /// Returns true if any of the columns of the primary key changed type.
    fn primary_key_column_changed(&self, previous_pk: &PrimaryKey) -> bool {
        self.column_pairs()
            .filter(|columns| {
                previous_pk
                    .columns
                    .iter()
                    .any(|pk_col| pk_col.name() == columns.previous.name())
            })
            .any(|columns| self.db.column_changes_for_walkers(columns).type_changed())
    }

    fn previous_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous().foreign_keys()
    }

    fn next_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next().foreign_keys()
    }

    fn previous_indexes(&self) -> impl Iterator<Item = IndexWalker<'schema>> {
        self.previous().indexes()
    }

    fn next_indexes(&self) -> impl Iterator<Item = IndexWalker<'schema>> {
        self.next().indexes()
    }

    pub(super) fn previous(&self) -> TableWalker<'schema> {
        self.tables.previous
    }

    pub(super) fn next(&self) -> TableWalker<'schema> {
        self.tables.next
    }

    pub(super) fn table_ids(&self) -> Pair<TableId> {
        self.tables.map(|t| t.id)
    }
}

/// Compare two SQL indexes and return whether they only differ by name.
fn indexes_match(first: IndexWalker<'_>, second: IndexWalker<'_>, flavour: &dyn SqlFlavour) -> bool {
    let left_cols = first.columns();
    let right_cols = second.columns();

    left_cols.len() == right_cols.len()
        && left_cols.zip(right_cols).all(|(a, b)| {
            let names_match = a.as_column().name() == b.as_column().name();
            let lengths_match = a.length() == b.length();
            let orders_match = a.sort_order().unwrap_or_default() == b.sort_order().unwrap_or_default();

            names_match && lengths_match && orders_match
        })
        && first.index_type() == second.index_type()
        && flavour.indexes_match(first, second)
}
