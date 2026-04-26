use std::sync::atomic::{AtomicU64, Ordering};

/// Simple operational counters emitted to logs (`target=metrics`).
#[derive(Debug, Default)]
pub struct GatewayMetrics {
    mir_started: AtomicU64,
    mir_reused: AtomicU64,
    mir_linked: AtomicU64,
    mir_success: AtomicU64,
    mir_failures: AtomicU64,
    dedup_hits: AtomicU64,
    dedup_misses: AtomicU64,
    rdf_invalid_payloads: AtomicU64,
    zip_entries: AtomicU64,
    zip_reused: AtomicU64,
}

impl GatewayMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    fn bump(counter: &AtomicU64) -> u64 {
        counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn record_mir_started(&self, file_id: i64, context_id: i32) {
        let total = Self::bump(&self.mir_started);
        tracing::info!(target: "metrics", metric = "mir_started_total", total, file_id, context_id);
    }

    pub fn record_mir_reused(&self, file_id: i64, context_id: i32) {
        let total = Self::bump(&self.mir_reused);
        tracing::info!(target: "metrics", metric = "mir_reused_total", total, file_id, context_id);
    }

    pub fn record_mir_linked(&self, file_id: i64, context_id: i32) {
        let total = Self::bump(&self.mir_linked);
        tracing::info!(target: "metrics", metric = "mir_linked_total", total, file_id, context_id);
    }

    pub fn record_mir_success(&self, file_id: i64, context_id: i32) {
        let total = Self::bump(&self.mir_success);
        tracing::info!(target: "metrics", metric = "mir_success_total", total, file_id, context_id);
    }

    pub fn record_mir_failure(&self, file_id: i64, context_id: i32) {
        let total = Self::bump(&self.mir_failures);
        tracing::warn!(target: "metrics", metric = "mir_failure_total", total, file_id, context_id);
    }

    pub fn record_dedup_hit(&self, file_id: i64, context_id: i32, sha_hex: &str) {
        let total = Self::bump(&self.dedup_hits);
        tracing::info!(
            target: "metrics",
            metric = "files_dedup_hits_total",
            total,
            file_id,
            context_id,
            sha_hex
        );
    }

    pub fn record_dedup_miss(&self, file_id: i64, context_id: i32, sha_hex: &str) {
        let total = Self::bump(&self.dedup_misses);
        tracing::info!(
            target: "metrics",
            metric = "files_dedup_misses_total",
            total,
            file_id,
            context_id,
            sha_hex
        );
    }

    pub fn record_rdf_invalid_payload(&self, topic: &str) {
        let total = Self::bump(&self.rdf_invalid_payloads);
        tracing::warn!(
            target: "metrics",
            metric = "rdf_invalid_payload_total",
            total,
            topic
        );
    }

    pub fn record_zip_entry(&self, file_id: i64, context_id: i32, reused: bool) {
        let total = Self::bump(&self.zip_entries);
        tracing::info!(
            target: "metrics",
            metric = "zip_entries_total",
            total,
            file_id,
            context_id,
            reused
        );
        if reused {
            let reused_total = Self::bump(&self.zip_reused);
            tracing::info!(
                target: "metrics",
                metric = "zip_reused_total",
                total = reused_total,
                file_id,
                context_id
            );
        }
    }
}
