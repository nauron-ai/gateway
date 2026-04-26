use uuid::Uuid;

pub(crate) fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("uuid")
}
