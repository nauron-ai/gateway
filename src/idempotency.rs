use uuid::Uuid;

const NAMESPACE_BYTES: [u8; 16] = [
    0x7d, 0x31, 0x2a, 0xc2, 0x17, 0x1d, 0x4c, 0x6f, 0x88, 0x2b, 0xa2, 0x66, 0x0a, 0x6c, 0x92, 0x3f,
];

pub fn build_deterministic_job_id(prefix: &str, context_id: i32, user_id: &str, key: &str) -> Uuid {
    let namespace = Uuid::from_bytes(NAMESPACE_BYTES);
    let name = build_name(prefix, context_id, user_id, key);
    Uuid::new_v5(&namespace, &name)
}

fn build_name(prefix: &str, context_id: i32, user_id: &str, key: &str) -> Vec<u8> {
    let mut name = Vec::with_capacity(prefix.len() + user_id.len() + key.len() + 28);
    push_segment(&mut name, prefix.as_bytes());
    name.extend_from_slice(&context_id.to_be_bytes());
    push_segment(&mut name, user_id.as_bytes());
    push_segment(&mut name, key.as_bytes());
    name
}

fn push_segment(buffer: &mut Vec<u8>, segment: &[u8]) {
    buffer.extend_from_slice(&(segment.len() as u64).to_be_bytes());
    buffer.extend_from_slice(segment);
}

#[cfg(test)]
mod tests {
    use super::build_deterministic_job_id;

    #[test]
    fn deterministic_job_id_distinguishes_embedded_delimiters() {
        let left = build_deterministic_job_id("ingest", 7, "a:b", "c");
        let right = build_deterministic_job_id("ingest", 7, "a", "b:c");

        assert_ne!(left, right);
    }

    #[test]
    fn deterministic_job_id_is_stable() {
        let id = build_deterministic_job_id("ingest", 42, "user@example.com", "my-key");

        assert_eq!(id.to_string(), "95429cd5-945a-5707-823c-305c423ba3ce");
    }
}
