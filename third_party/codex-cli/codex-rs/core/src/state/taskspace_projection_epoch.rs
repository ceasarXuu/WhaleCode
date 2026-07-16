use codex_protocol::models::ResponseItem;
use sha2::Digest;
use sha2::Sha256;

#[derive(Debug, Clone)]
pub(crate) struct TaskSpaceProviderProjectionEpoch {
    pub(crate) scope: String,
    pub(crate) context: String,
    pub(crate) anchor: usize,
    prefix_sha256: String,
}

impl TaskSpaceProviderProjectionEpoch {
    pub(crate) fn new(
        scope: String,
        context: String,
        anchor: usize,
        items: &[ResponseItem],
    ) -> Result<Self, String> {
        Ok(Self {
            scope,
            context,
            anchor,
            prefix_sha256: prefix_sha256(items, anchor)?,
        })
    }

    fn prefix_matches(&self, items: &[ResponseItem]) -> Result<bool, String> {
        if self.anchor > items.len() {
            return Ok(false);
        }
        Ok(prefix_sha256(items, self.anchor)? == self.prefix_sha256)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TaskSpaceProjectionEpochDecision {
    Reuse { context: String, anchor: usize },
    Refresh { anchor: usize, reason: &'static str },
}

pub(crate) fn decide_taskspace_projection_epoch(
    epoch: Option<&TaskSpaceProviderProjectionEpoch>,
    scope: &str,
    items: &[ResponseItem],
) -> Result<TaskSpaceProjectionEpochDecision, String> {
    let Some(epoch) = epoch else {
        return Ok(TaskSpaceProjectionEpochDecision::Refresh {
            anchor: items.len(),
            reason: "epoch_missing",
        });
    };
    if !epoch.prefix_matches(items)? {
        return Ok(TaskSpaceProjectionEpochDecision::Refresh {
            anchor: items.len(),
            reason: "anchor_prefix_changed",
        });
    }
    if epoch.scope != scope {
        return Ok(TaskSpaceProjectionEpochDecision::Refresh {
            anchor: epoch.anchor,
            reason: "projection_scope_changed",
        });
    }
    Ok(TaskSpaceProjectionEpochDecision::Reuse {
        context: epoch.context.clone(),
        anchor: epoch.anchor,
    })
}

fn prefix_sha256(items: &[ResponseItem], anchor: usize) -> Result<String, String> {
    let prefix = items
        .get(..anchor)
        .ok_or_else(|| format!("projection anchor {anchor} exceeds {} items", items.len()))?;
    let bytes = serde_json::to_vec(prefix)
        .map_err(|error| format!("projection epoch prefix serialization failed: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
#[path = "taskspace_projection_epoch_tests.rs"]
mod tests;
