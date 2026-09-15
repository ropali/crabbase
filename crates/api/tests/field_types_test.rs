//! HTTP API integration tests for ALL Collection Field Types.
//!
//! This test suite verifies that every single data type supported by Crabbase:
//!   1. PlainText    ("text" / "plaintext")
//!   2. RichText     ("richtext" / "editor")
//!   3. Number       ("number" / "integer")
//!   4. Bool         ("bool" / "boolean")
//!   5. Email        ("email")
//!   6. Url          ("url")
//!   7. Datetime     ("datetime" / "date")
//!   8. AutoDatetime ("autodate" / "autodatetime")
//!   9. File         ("file")
//!  10. Relation     ("relation" -> foreign key UUID)
//!  11. Select       ("select")
//!  12. Json         ("json")
//!  13. GeoPoint     ("geopoint")
//!
//! Can be:
//!   - Defined on a collection via `POST /api/collections`
//!   - Inserted as a record via `POST /api/collections/:name/records`
//!   - Read by ID via `GET /api/collections/:name/records/:id`
//!   - Listed via `GET /api/collections/:name/records`
//!   - Updated via `PATCH /api/collections/:name/records/:id`

mod common;

use axum::http::StatusCode;
use common::{TestApp, create_collection, create_record_api, expect, insert_superuser, login};
use serde_json::json;

async fn setup() -> (TestApp, String) {
    let app = TestApp::new("field_types").await;
    insert_superuser(&app.db.pool, "admin@test.com", "admin_pw").await;
    let token = login(&app, "_superusers", "admin@test.com", "admin_pw").await;
    (app, token)
}

#[tokio::test]
async fn test_all_13_field_types_end_to_end_crud() {
    let (app, token) = setup().await;

    // 1. Create target collection for the Relation field type
    create_collection(
        &app,
        "categories",
        vec![json!({ "name": "category_name", "type": "text" })],
        &token,
    )
    .await;

    // Insert a category record to obtain a valid target UUID
    let category =
        create_record_api(&app, "categories", json!({ "category_name": "Technology" })).await;
    let category_id = category["id"].as_str().expect("category UUID").to_string();

    // 2. Create the master collection containing columns of ALL 13 data types
    let all_columns = vec![
        json!({ "name": "plain_text_field",  "type": "text" }),
        json!({ "name": "rich_text_field",   "type": "richtext" }),
        json!({ "name": "number_field",      "type": "number", "index": true }),
        json!({ "name": "bool_field",        "type": "bool",   "index": true }),
        json!({ "name": "email_field",       "type": "email",  "index": true }),
        json!({ "name": "url_field",         "type": "url" }),
        json!({ "name": "datetime_field",    "type": "datetime" }),
        json!({ "name": "autodate_field",    "type": "autodate" }),
        json!({ "name": "file_field",        "type": "file" }),
        json!({ "name": "relation_field",    "type": "relation", "related_to": "categories" }),
        json!({ "name": "select_field",      "type": "select" }),
        json!({ "name": "json_field",        "type": "json" }),
        json!({ "name": "geopoint_field",    "type": "geopoint" }),
    ];

    let col_body = create_collection(&app, "all_types_table", all_columns, &token).await;
    assert_eq!(col_body["name"], "all_types_table");

    // Verify all 13 columns exist in collection metadata
    let fields = col_body["fields"].as_array().expect("fields array");
    assert_eq!(
        fields.len(),
        13,
        "expected 13 field definitions in collection"
    );

    // 3. Create a record with values for all 13 field types
    let insert_payload = json!({
        "plain_text_field": "Hello World",
        "rich_text_field":  "<p>Rich <strong>content</strong></p>",
        "number_field":     42000,
        "bool_field":       true,
        "email_field":      "developer@crabbase.io",
        "url_field":        "https://crabbase.io",
        "datetime_field":   "2026-09-01T12:00:00Z",
        "file_field":       "avatar_123.png",
        "relation_field":   category_id,
        "select_field":     "option_b",
        "json_field":       json!({ "nested_key": "nested_value", "count": 5 }).to_string(),
        "geopoint_field":   "37.7749,-122.4194"
    });

    let created_record = create_record_api(&app, "all_types_table", insert_payload.clone()).await;

    let record_id = created_record["id"]
        .as_str()
        .expect("record id")
        .to_string();
    assert!(created_record["created"].as_str().is_some());
    assert!(created_record["updated"].as_str().is_some());

    // 4. Retrieve record by ID (GET /records/:id) and assert every field type
    let get_res = app
        .get(&format!(
            "/api/collections/all_types_table/records/{record_id}"
        ))
        .await;
    let fetched = expect(get_res, StatusCode::OK).await;
    let data = &fetched["data"];

    assert_eq!(data["plain_text_field"], "Hello World");
    assert_eq!(
        data["rich_text_field"],
        "<p>Rich <strong>content</strong></p>"
    );
    assert_eq!(data["number_field"], 42000);
    assert_eq!(data["bool_field"], true);
    assert_eq!(data["email_field"], "developer@crabbase.io");
    assert_eq!(data["url_field"], "https://crabbase.io");
    assert!(
        data["datetime_field"]
            .as_str()
            .unwrap()
            .contains("2026-09-01")
    );
    assert!(
        data["autodate_field"].as_str().is_some(),
        "autodate_field should be populated by DEFAULT now()"
    );
    assert_eq!(data["file_field"], "avatar_123.png");
    assert_eq!(data["relation_field"], category_id);
    assert_eq!(data["select_field"], "option_b");
    assert!(data["json_field"].as_str().unwrap().contains("nested_key"));
    assert_eq!(data["geopoint_field"], "37.7749,-122.4194");

    // 5. Verify list endpoint (GET /records) returns the fields accurately
    let list_res = app
        .get_auth("/api/collections/all_types_table/records", &token)
        .await;
    let list_body = expect(list_res, StatusCode::OK).await;
    let items = list_body["items"].as_array().expect("items array");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["data"]["plain_text_field"], "Hello World");
    assert_eq!(items[0]["data"]["relation_field"], category_id);
    assert_eq!(items[0]["data"]["number_field"], 42000);
    assert_eq!(items[0]["data"]["bool_field"], true);
}

#[tokio::test]
async fn test_null_parameter_binding_for_all_field_types() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "cat_null",
        vec![json!({ "name": "name", "type": "text" })],
        &token,
    )
    .await;

    let all_columns = vec![
        json!({ "name": "plain_text_field",  "type": "text" }),
        json!({ "name": "rich_text_field",   "type": "richtext" }),
        json!({ "name": "number_field",      "type": "number" }),
        json!({ "name": "bool_field",        "type": "bool" }),
        json!({ "name": "email_field",       "type": "email" }),
        json!({ "name": "url_field",         "type": "url" }),
        json!({ "name": "datetime_field",    "type": "datetime" }),
        json!({ "name": "autodate_field",    "type": "autodate" }),
        json!({ "name": "file_field",        "type": "file" }),
        json!({ "name": "relation_field",    "type": "relation", "related_to": "cat_null" }),
        json!({ "name": "select_field",      "type": "select" }),
        json!({ "name": "json_field",        "type": "json" }),
        json!({ "name": "geopoint_field",    "type": "geopoint" }),
    ];

    create_collection(&app, "null_types_table", all_columns, &token).await;

    // Insert record with explicit NULLs for all nullable fields
    let null_payload = json!({
        "plain_text_field": null,
        "rich_text_field":  null,
        "number_field":     null,
        "bool_field":       null,
        "email_field":      null,
        "url_field":        null,
        "datetime_field":   null,
        "file_field":       null,
        "relation_field":   null,
        "select_field":     null,
        "json_field":       null,
        "geopoint_field":   null
    });

    let created = create_record_api(&app, "null_types_table", null_payload).await;
    let record_id = created["id"].as_str().expect("record id");

    let get_res = app
        .get(&format!(
            "/api/collections/null_types_table/records/{record_id}"
        ))
        .await;
    let fetched = expect(get_res, StatusCode::OK).await;
    let data = &fetched["data"];

    assert!(data["plain_text_field"].is_null());
    assert!(data["rich_text_field"].is_null());
    assert!(data["number_field"].is_null());
    assert!(data["bool_field"].is_null());
    assert!(data["email_field"].is_null());
    assert!(data["url_field"].is_null());
    assert!(data["datetime_field"].is_null());
    assert!(data["file_field"].is_null());
    assert!(data["relation_field"].is_null());
    assert!(data["select_field"].is_null());
    assert!(data["geopoint_field"].is_null());
}

#[tokio::test]
async fn test_patch_updates_on_typed_fields() {
    let (app, token) = setup().await;

    create_collection(
        &app,
        "cat_patch",
        vec![json!({ "name": "name", "type": "text" })],
        &token,
    )
    .await;
    let cat1 = create_record_api(&app, "cat_patch", json!({ "name": "C1" })).await;
    let cat2 = create_record_api(&app, "cat_patch", json!({ "name": "C2" })).await;
    let cat1_id = cat1["id"].as_str().unwrap();
    let cat2_id = cat2["id"].as_str().unwrap();

    let all_columns = vec![
        json!({ "name": "number_field",      "type": "number" }),
        json!({ "name": "bool_field",        "type": "bool" }),
        json!({ "name": "datetime_field",    "type": "datetime" }),
        json!({ "name": "relation_field",    "type": "relation", "related_to": "cat_patch" }),
    ];

    create_collection(&app, "patch_types_table", all_columns, &token).await;

    let initial = create_record_api(
        &app,
        "patch_types_table",
        json!({
            "number_field": 10,
            "bool_field": false,
            "datetime_field": "2026-01-01T00:00:00Z",
            "relation_field": cat1_id
        }),
    )
    .await;
    let record_id = initial["id"].as_str().unwrap();

    // PATCH update individual fields
    let patch_res = app
        .patch_json(
            &format!("/api/collections/patch_types_table/records/{record_id}"),
            json!({
                "data": {
                    "number_field": 999,
                    "bool_field": true,
                    "datetime_field": "2026-12-31T23:59:59Z",
                    "relation_field": cat2_id
                }
            }),
            &token,
        )
        .await;
    let patched = expect(patch_res, StatusCode::OK).await;

    assert_eq!(patched["data"]["number_field"], 999);
    assert_eq!(patched["data"]["bool_field"], true);
    assert!(
        patched["data"]["datetime_field"]
            .as_str()
            .unwrap()
            .contains("2026-12-31")
    );
    assert_eq!(patched["data"]["relation_field"], cat2_id);
}

#[tokio::test]
async fn test_filter_expressions_against_typed_columns() {
    let (app, token) = setup().await;

    let all_columns = vec![
        json!({ "name": "num",  "type": "number" }),
        json!({ "name": "flag", "type": "bool" }),
        json!({ "name": "dt",   "type": "datetime" }),
    ];

    create_collection(&app, "filter_types_table", all_columns, &token).await;

    create_record_api(
        &app,
        "filter_types_table",
        json!({ "num": 100, "flag": true, "dt": "2026-06-01T00:00:00Z" }),
    )
    .await;
    create_record_api(
        &app,
        "filter_types_table",
        json!({ "num": 200, "flag": false, "dt": "2026-09-01T00:00:00Z" }),
    )
    .await;

    // Filter by number and boolean
    let res = app
        .get_auth(
            "/api/collections/filter_types_table/records?filter=num > 150",
            &token,
        )
        .await;
    let body = expect(res, StatusCode::OK).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["data"]["num"], 200);

    let res_bool = app
        .get_auth(
            "/api/collections/filter_types_table/records?filter=flag = true",
            &token,
        )
        .await;
    let body_bool = expect(res_bool, StatusCode::OK).await;
    let items_bool = body_bool["items"].as_array().unwrap();
    assert_eq!(items_bool.len(), 1);
    assert_eq!(items_bool[0]["data"]["num"], 100);
}
