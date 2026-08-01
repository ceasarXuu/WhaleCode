use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use tracing::Event;
use tracing::Subscriber;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

use super::emit_taskspace_response_finalized;
use crate::action_map::ActionMapPreparedCall;
use crate::action_map::ActionMapPreparedResponse;
use crate::action_map::ActionMapResponseSettlement;

#[derive(Default)]
struct FieldVisitor(BTreeMap<String, String>);

impl Visit for FieldVisitor {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

#[derive(Clone)]
struct EventCollector {
    fields: Arc<Mutex<BTreeMap<String, String>>>,
}

impl<S> Layer<S> for EventCollector
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "codex_core::taskspace" {
            return;
        }
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        if visitor.0.get("event_name").map(String::as_str) == Some("taskspace_response_finalized") {
            *self.fields.lock().unwrap() = visitor.0;
        }
    }
}

fn prepared_response() -> ActionMapPreparedResponse {
    ActionMapPreparedResponse {
        map_id: "map-log".to_string(),
        revision_before: 6,
        revision_after: 7,
        action: "execute",
        prepared_calls: vec![ActionMapPreparedCall {
            map_id: "map-log".to_string(),
            revision: 7,
            call_id: "ordinary-call".to_string(),
            call_index: 0,
            node_id: "work".to_string(),
            tool_name: "exec_command".to_string(),
            reservation_id: "reservation-log".to_string(),
        }],
    }
}

#[test]
fn finalization_log_records_mechanical_identity_without_tool_content() {
    let fields = Arc::new(Mutex::new(BTreeMap::new()));
    let _guard = tracing_subscriber::registry()
        .with(EventCollector {
            fields: Arc::clone(&fields),
        })
        .set_default();
    let prepared = prepared_response();
    let settlement = ActionMapResponseSettlement {
        map_id: prepared.map_id.clone(),
        reservation_revision_after: prepared.revision_after,
        canonical_revision: Some(8),
        prepared_action_count: 1,
        attributed_result_count: 0,
        outstanding_reservation_count: 1,
        error: None,
    };

    emit_taskspace_response_finalized(&prepared, &settlement, "control-log");

    let fields = fields.lock().unwrap();
    assert_eq!(
        fields.get("control_call_id").map(String::as_str),
        Some("control-log")
    );
    assert_eq!(fields.get("map_id").map(String::as_str), Some("map-log"));
    assert_eq!(
        fields.get("prepare_revision").map(String::as_str),
        Some("7")
    );
    assert_eq!(
        fields.get("canonical_revision").map(String::as_str),
        Some("8")
    );
    assert_eq!(
        fields.get("status").map(String::as_str),
        Some("settlement_incomplete")
    );
    assert_eq!(
        fields.get("reason_code").map(String::as_str),
        Some("taskspace_response_attribution_incomplete")
    );
    for forbidden in ["goal", "command", "tool", "arguments", "output"] {
        assert!(!fields.contains_key(forbidden));
    }
}
