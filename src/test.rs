use tantivy::schema::{Schema, INDEXED, STORED, TEXT};

pub fn make_test_schema() -> Schema {
    let mut schema = Schema::builder();
    schema.add_u64_field("id", STORED | INDEXED);
    schema.add_text_field("text", STORED | TEXT);
    schema.build()
}
