pub const SCHEMA_TEXT: &str = include_str!("../skills_schema.graphql");

pub fn schema() -> trustfall::Schema {
    trustfall::Schema::parse(SCHEMA_TEXT).expect("invalid skills schema")
}
