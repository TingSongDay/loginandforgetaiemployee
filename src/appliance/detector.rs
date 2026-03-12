use crate::appliance::{
    dedupe_store::WorkerDedupeState, messages::MessageRecord, platforms::MessageDirection,
};

pub fn detect_new_inbound_message(
    state: &WorkerDedupeState,
    records: &[MessageRecord],
) -> Option<MessageRecord> {
    if !state.bootstrapped {
        return None;
    }

    records
        .iter()
        .rev()
        .find(|record| {
            record.direction == MessageDirection::Inbound
                && state.last_seen_inbound_message_id.as_deref()
                    != Some(record.platform_message_id.as_str())
                && !is_processed(state, record)
                && !is_pending(state, record)
        })
        .cloned()
}

fn is_processed(state: &WorkerDedupeState, record: &MessageRecord) -> bool {
    state
        .processed_message_ids
        .iter()
        .any(|message_id| message_id == &record.platform_message_id)
        || state
            .processed_message_fingerprints
            .iter()
            .any(|fingerprint| fingerprint == &record.raw_fingerprint)
        || state.last_processed_message_id.as_deref() == Some(&record.platform_message_id)
        || state.last_processed_message_fingerprint.as_deref() == Some(&record.raw_fingerprint)
}

fn is_pending(state: &WorkerDedupeState, record: &MessageRecord) -> bool {
    state.pending_reply.as_ref().is_some_and(|pending| {
        pending.inbound_message_id == record.platform_message_id
            || pending.inbound_fingerprint == record.raw_fingerprint
    })
}
