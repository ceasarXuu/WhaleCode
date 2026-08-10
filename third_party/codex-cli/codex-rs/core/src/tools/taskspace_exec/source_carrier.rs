use serde::Deserialize;

use super::TaskSpaceExecCatalog;
use super::TaskSpaceExecPlan;
use super::TaskSpaceExecPlanDecodeError;

const SOURCE_PREFIX: &str = "taskspace.plan(";
const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceArguments {
    source: String,
}

pub(super) fn decode(
    arguments: &str,
    catalog: &TaskSpaceExecCatalog,
) -> Result<TaskSpaceExecPlan, TaskSpaceExecPlanDecodeError> {
    let arguments = serde_json::from_str::<SourceArguments>(arguments)
        .map_err(|error| invalid(format!("source carrier arguments are invalid: {error}")))?;
    let source = arguments.source;
    if source.len() > MAX_SOURCE_BYTES {
        return Err(invalid(format!(
            "source carrier is {} bytes; maximum is {MAX_SOURCE_BYTES}",
            source.len()
        )));
    }

    let source = source.trim();
    if source.is_empty() {
        return Err(invalid("source carrier is empty"));
    }
    if source.starts_with("```") || source.ends_with("```") {
        return Err(invalid("source carrier must not use Markdown fences"));
    }
    let Some(body) = source.strip_prefix(SOURCE_PREFIX) else {
        return Err(invalid("source carrier must start with `taskspace.plan(`"));
    };
    let body = body.strip_suffix(';').unwrap_or(body).trim_end();
    let Some(plan_json) = body.strip_suffix(')') else {
        return Err(invalid(
            "source carrier must end with `)` or `);` and contain no trailing statements",
        ));
    };

    TaskSpaceExecPlan::decode(plan_json.trim(), catalog)
}

fn invalid(message: impl Into<String>) -> TaskSpaceExecPlanDecodeError {
    TaskSpaceExecPlanDecodeError::InvalidJson(message.into())
}

pub(super) fn encode(plan: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "source": format!("taskspace.plan({});", plan)
    })
}
