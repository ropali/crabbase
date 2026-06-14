#[derive(Debug, Clone, PartialEq)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub collection_type: String,
    pub records: u32,
    pub modified: String,
}
