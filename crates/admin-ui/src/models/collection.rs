#[derive(Debug, Clone, PartialEq)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub system: bool,
    pub fields: serde_json::Value,
}
