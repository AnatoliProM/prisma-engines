use super::*;

pub fn convert_introspected_columns(
    columns: Vec<IntrospectedColumn>,
    foreign_keys: Vec<IntrospectedForeignKey>,
    column_type: Box<dyn Fn(&IntrospectedColumn) -> ColumnType>,
) -> Vec<Column> {
    columns
        .iter()
        .map(|c| {
            let foreign_key = foreign_keys
                .iter()
                .find(|fk| fk.column == c.name && fk.table == c.table)
                .map(|fk| ForeignKey {
                    name: Some(fk.name.clone()),
                    table: fk.referenced_table.clone(),
                    column: fk.referenced_column.clone(),
                    on_delete: OnDelete::NoAction, // TODO:: fix this hardcoded value
                });
            Column {
                name: c.name.clone(),
                tpe: column_type(c),
                is_required: c.is_required,
                foreign_key,
                sequence: None,
                default: None,
            }
        })
        .collect()
}

#[derive(Debug)]
pub struct IntrospectedForeignKey {
    pub name: String,
    pub table: String,
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Debug, Clone)]
pub struct IntrospectedColumn {
    pub name: String,
    pub table: String,
    pub tpe: String,
    pub default: Option<String>,
    pub is_required: bool,
    pub pk: u32,
}
