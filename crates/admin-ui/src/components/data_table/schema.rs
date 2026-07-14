use crate::components::data_table::column::{
    ColumnDef, bool_render, code_render, date_render, default_render, id_render,
    nullable_text_render,
};
use crate::models::collection::Collection;

pub fn columns_from_schema(schema: &Collection) -> Vec<ColumnDef> {
    let mut cols = Vec::new();

    // Add "id" column first
    cols.push(
        ColumnDef::new("id", "id")
            .render(id_render)
            .icon("key")
            .sortable(),
    );

    // Add dynamically each field of the collection
    for field in &schema.fields {
        let key = field.name.clone();
        let header = field.name.clone();

        // Choose icon depending on data type
        let icon = match field.data_type.as_str() {
            "Email" => Some("mail"),
            "Bool" => Some("check_box"),
            "Number" => Some("tag"),
            "Relation" => Some("link"),
            "Datetime" | "AutoDatetime" => Some("schedule"),
            _ => Some("text_fields"),
        };

        let data_type = field.data_type.clone();
        let mut col_def =
            ColumnDef::new(&key, &header).render(move |cell_val| match data_type.as_str() {
                "Bool" => bool_render(cell_val),
                "Email" => nullable_text_render(cell_val),
                "Number" => default_render(cell_val),
                "Relation" => code_render(cell_val),
                "Datetime" | "AutoDatetime" => date_render(cell_val),
                _ => nullable_text_render(cell_val),
            });

        if let Some(ic) = icon {
            col_def = col_def.icon(ic);
        }
        cols.push(col_def);
    }

    // Add "created" column
    cols.push(
        ColumnDef::new("created", "created")
            .render(date_render)
            .icon("schedule")
            .sortable(),
    );

    // Add "updated" column
    cols.push(
        ColumnDef::new("updated", "updated")
            .render(date_render)
            .icon("update")
            .sortable(),
    );

    cols
}
