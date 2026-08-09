use crate::components::data_table::column::{
    ColumnDef, bool_render, clipped_text_render, code_render, date_render, default_render,
    id_render, json_render, richtext_render,
};
use crate::models::collection::Collection;

pub fn columns_from_schema(schema: &Collection) -> Vec<ColumnDef> {
    let mut cols = Vec::new();

    // Add "id" column first
    cols.push(
        ColumnDef::new("id", "id")
            .data_type("id")
            .render(id_render)
            .icon("key")
            .sortable(),
    );

    // Add dynamically each field of the collection
    for field in &schema.fields {
        let key = field.name.clone();
        let header = field.name.clone();
        let data_type = field.data_type.clone();

        // Choose icon depending on data type
        let icon = match data_type.to_lowercase().as_str() {
            "email" => Some("mail"),
            "bool" => Some("check_box"),
            "number" => Some("tag"),
            "relation" => Some("link"),
            "datetime" | "autodatetime" => Some("schedule"),
            "json" => Some("data_object"),
            "richtext" | "editor" => Some("article"),
            _ => Some("text_fields"),
        };

        let key_clone = key.clone();
        let dt_clone = data_type.clone();

        let mut col_def =
            ColumnDef::new(&key, &header)
                .data_type(&data_type)
                .render(
                    move |cell_val, on_view| match dt_clone.to_lowercase().as_str() {
                        "bool" => bool_render(cell_val, on_view),
                        "number" => default_render(cell_val, on_view),
                        "relation" => code_render(cell_val, on_view),
                        "datetime" | "autodatetime" => date_render(cell_val, on_view),
                        "json" => json_render(key_clone.clone(), cell_val, on_view),
                        "richtext" | "editor" => {
                            richtext_render(key_clone.clone(), cell_val, on_view)
                        }
                        _ => clipped_text_render(
                            key_clone.clone(),
                            dt_clone.clone(),
                            cell_val,
                            on_view,
                        ),
                    },
                );

        if let Some(ic) = icon {
            col_def = col_def.icon(ic);
        }
        cols.push(col_def);
    }

    // Add "created" column
    cols.push(
        ColumnDef::new("created", "created")
            .data_type("datetime")
            .render(date_render)
            .icon("schedule")
            .sortable(),
    );

    // Add "updated" column
    cols.push(
        ColumnDef::new("updated", "updated")
            .data_type("datetime")
            .render(date_render)
            .icon("update")
            .sortable(),
    );

    cols
}
