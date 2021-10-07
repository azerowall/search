use tantivy::chrono;
use tantivy::schema::{Field, FieldType, Term};

pub fn make_term(field: Field, field_type: &FieldType, value: &str) -> crate::Result<Term> {
    Ok(match field_type {
        FieldType::Str(_) => Term::from_field_text(field, value),
        FieldType::U64(_) => Term::from_field_u64(field, value.parse()?),
        FieldType::I64(_) => Term::from_field_i64(field, value.parse()?),
        FieldType::F64(_) => Term::from_field_f64(field, value.parse()?),
        FieldType::Date(_) => {
            let date = chrono::DateTime::parse_from_rfc3339(value)?
                .with_timezone(&chrono::Utc);
            Term::from_field_date(field, &date)
        },
        FieldType::Bytes(_) => Term::from_field_bytes(field, &base64::decode(value)?),
        FieldType::HierarchicalFacet(_) => todo!(),
    })
}