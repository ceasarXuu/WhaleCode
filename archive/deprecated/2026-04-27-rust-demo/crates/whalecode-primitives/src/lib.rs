use std::collections::{BTreeMap, BTreeSet};
use whalecode_protocol::{
    ArtifactSchemaRef, EventSchemaRef, GateSpec, PermissionOverlaySpec, PhaseHookSpec,
    PrimitiveEvalSpec, PrimitiveId, PrimitiveManifest, ReplayReducerSpec, ViewerTriggerSpec,
};

pub trait PrimitiveModule {
    fn manifest(&self) -> PrimitiveManifest;

    fn artifact_schemas(&self) -> Vec<ArtifactSchemaRef> {
        Vec::new()
    }

    fn event_schemas(&self) -> Vec<EventSchemaRef> {
        Vec::new()
    }

    fn gates(&self) -> Vec<GateSpec> {
        Vec::new()
    }

    fn phase_hooks(&self) -> Vec<PhaseHookSpec> {
        Vec::new()
    }

    fn permission_overlays(&self) -> Vec<PermissionOverlaySpec> {
        Vec::new()
    }

    fn replay_reducers(&self) -> Vec<ReplayReducerSpec> {
        Vec::new()
    }

    fn viewer_triggers(&self) -> Vec<ViewerTriggerSpec> {
        Vec::new()
    }

    fn eval_specs(&self) -> Vec<PrimitiveEvalSpec> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct PrimitiveRegistry {
    manifests: BTreeMap<PrimitiveId, PrimitiveManifest>,
    enabled: BTreeSet<PrimitiveId>,
}

impl PrimitiveRegistry {
    pub fn register_manifest(&mut self, manifest: PrimitiveManifest) {
        if manifest.default_enabled {
            self.enabled.insert(manifest.id.clone());
        }
        self.manifests.insert(manifest.id.clone(), manifest);
    }

    pub fn enable(&mut self, id: &PrimitiveId) -> bool {
        if self.manifests.contains_key(id) {
            self.enabled.insert(id.clone());
            true
        } else {
            false
        }
    }

    pub fn disable(&mut self, id: &PrimitiveId) {
        self.enabled.remove(id);
    }

    pub fn is_enabled(&self, id: &PrimitiveId) -> bool {
        self.enabled.contains(id)
    }
}
