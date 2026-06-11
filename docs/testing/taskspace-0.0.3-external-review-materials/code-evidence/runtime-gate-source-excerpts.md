# Runtime Gate Source Excerpts

## finish_main_node

```rust
                  TaskStatus::Pending,
                ));
            }
        }
        let task = self
            .tasks
            .get_mut(task_id)
            .expect("target TaskSpace task was validated before routing");
        if task.owner_session_id.is_none() {
            task.owner_session_id = Some(owner_session_id);
        }
        let previous_status = task.status;
        task.status = TaskStatus::Active;
        if previous_status != TaskStatus::Active {
            events.push(task_status_changed_event(
                task_id,
                previous_status,
                TaskStatus::Active,
            ));
        }
        self.active_task_id = Some(task_id.to_string());
        self.active_map_id = Some(target_map_id.clone());
        self.current_main_node_id = None;
        self.current_main_lease_id = None;
        self.mark_routing_complete();
        events.push(MapRuntimeEvent::TaskRouted(MapRuntimeTaskRoutedEvent {
            previous_task_id,
            current_task_id: task_id.to_string(),
            previous_map_id,
            current_map_id: target_map_id,
        }));
        Ok(events)
    }

    #[allow(dead_code)]
    pub(crate) fn finish_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        result_summary: String,
        next_node_id: Option<String>,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        self.finish_main_node_with_next(
            owner_session_id,
            node_id,
            result_summary,
            next_node_id,
            None,
        )
    }

    pub(crate) fn finish_main_node_with_next(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        result_summary: String,
        next_node_id: Option<String>,
        next_node_draft: Option<ActionMapNextNodeDraft>,
    ) -> Result<(ActionMapFinishNodeOutcome, Vec<MapRuntimeEvent>), String> {
        let result_summary = result_summary.trim();
        if result_summary.is_empty() {
            return Err("TaskSpace finish_node result_summary cannot be empty.".to_string());
        }
        let next_node_id = next_node_id
            .as_deref()
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty());
        if next_node_id.is_some() && next_node_draft.is_some() {
            return Err(
                "TaskSpace finish_node cannot provide both next_node_id and next node draft fields."
                    .to_string(),
            );
        }
        if let Some(next_node_id) = next_node_id {
            self.validate_next_main_binding_after_finish(node_id, next_node_id)?;
        }
        if let Some(draft) = next_node_draft.as_ref() {
            validate_live_node_kind(draft.kind)?;
            if draft.title.trim().is_empty() {
                return Err("TaskSpace next_node_title cannot be empty.".to_string());
            }
            if draft.context_summary.trim().is_empty() {
                return Err("TaskSpace next_node_context_summary cannot be empty.".to_string());
            }
            self.validate_next_node_draft_after_finish(node_id, draft)?;
        }
        self.validate_broad_inspect_finish_transition(
            owner_session_id,
            node_id,
            next_node_id,
            next_node_draft.as_ref(),
        )?;
        self.validate_completion_evidence(node_id)?;
        if self.active_node_kind(node_id)? == NodeKind::FinalSynthesis {
            self.validate_final_response_ready(owner_session_id, result_summary, false)?;
        }
        let (result_id, mut events) = self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Result,
            result_summary.to_string(),
            NodeStatus::Completed,
            true,
        )?;
        let mut bound_next_node_id = None;
        if let Some(next_node_id) = next_node_id {
            let bind_events = self.bind_main_node(owner_session_id, next_node_id)?;
            events.extend(bind_events);
            bound_next_node_id = Some(next_node_id.to_string());
        } else if let Some(draft) = next_node_draft {
            let dependency_node_ids = if draft.dependency_node_ids.is_empty() {
                vec![node_id.to_string()]
            } else {
                draft.dependency_node_ids
            };
            let (created_node_id, node_events) = self.create_node_for_main_with_kind(
                owner_session_id,
                draft.kind,
                draft.title,
                draft.context_summary,
                dependency_node_ids,
                true,
            )?;
            events.extend(node_events);
            bound_next_node_id = Some(created_node_id);
        }
        Ok((
            ActionMapFinishNodeOutcome {
                result_id,
                next_node_id: bound_next_node_id,
            },
            events,
        ))
    }

    pub(crate) fn block_main_node(
        &mut self,
        owner_session_id: ThreadId,
        node_id: &str,
        blocker_summary: String,
    ) -> Result<(NodeResultId, Vec<MapRuntimeEvent>), String> {
        let blocker_summary = blocker_summary.trim();
        if blocker_summary.is_empty() {
            return Err("TaskSpace block_node blocker_summary cannot be empty.".to_string());
        }
        self.record_main_node_lifecycle_result(
            owner_session_id,
            node_id,
            NodeResultKind::Blocker,
            blocker_summary.to_string(),
            NodeStatus::Blocked,
            false,
        )
    }

    pub(crate) fn record_output_contract_for_main(
        &mut self,
        owner_session_id: ThreadId,
        output_contract_id: &str,
        kind: &str,
        description: String,
        evidence_refs: Vec<ActionMapEvidenceRefInput>,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let (task_id, map_id) = self.active_task_context_for_cognitive_update(owner_session_id)?;
        let id = require_nonempty("output_contract_id", output_contract_id)?;
        let kind = OutputContractKind::from_str(kind).ok_or_else(|| {
            "TaskSpace rec
```

## mark_result_validity_for_main

```rust
str,
        statement: String,
        evidence_refs: Vec<ActionMapEvidenceRefInput>,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let (task_id, map_id) = self.active_task_context_for_cognitive_update(owner_session_id)?;
        let id = require_nonempty("claim_id", claim_id)?;
        let statement = require_nonempty_owned("statement", statement)?;
        let evidence_refs = self.normalize_evidence_refs(&task_id, Some(&map_id), evidence_refs)?;
        self.validate_active_fact_evidence_refs(&task_id, Some(&map_id), &evidence_refs)?;

        let task = self
            .tasks
            .get_mut(&task_id)
            .expect("active task was validated before fact update");
        let record = CognitiveClaim {
            id: id.clone(),
            statement,
            evidence_refs,
        };
        upsert_cognitive_claim(&mut task.cognitive_state.facts, record);
        Ok(vec![MapRuntimeEvent::CognitiveStateUpdated(
            MapRuntimeCognitiveStateUpdatedEvent {
                task_id,
                map_id: Some(map_id),
                update_kind: "fact".to_string(),
                record_id: id,
            },
        )])
    }

    pub(crate) fn mark_result_validity_for_main(
        &mut self,
        owner_session_id: ThreadId,
        result_id: &str,
        validity: &str,
        validity_reason: String,
        claims: Vec<ActionMapCognitiveClaimInput>,
        evidence_refs: Vec<ActionMapEvidenceRefInput>,
        changed_artifacts: Vec<String>,
        validator_refs: Vec<String>,
        remaining_uncertainty: Vec<String>,
    ) -> Result<Vec<MapRuntimeEvent>, String> {
        let (task_id, map_id) = self.active_task_context_for_cognitive_update(owner_session_id)?;
        let result_id = require_nonempty("result_id", result_id)?;
        let validity = ResultValidity::from_str(validity).ok_or_else(|| {
            "TaskSpace mark_result_validity validity must be one of: unreviewed, accepted, questioned, invalid."
                .to_string()
        })?;
        let validity_reason = require_nonempty_owned("validity_reason", validity_reason)?;
        if !self
            .maps
            .get(&map_id)
            .is_some_and(|map| map.results.contains_key(&result_id))
        {
            return Err(format!(
                "TaskSpace result `{result_id}` does not exist on active task path `{map_id}`."
            ));
        }
        let evidence_refs =
            self.normalize_evidence_refs_for_result(&map_id, &result_id, evidence_refs)?;
        if evidence_refs.is_empty() {
            return Err(
                "TaskSpace mark_result_validity evidence_refs cannot be empty.".to_string(),
            );
        }
        let claims = self.normalize_claim_inputs_for_result(&map_id, &result_id, claims)?;
        if validity == ResultValidity::Accepted && claims.is_empty() {
            return Err(
                "TaskSpace mark_result_validity accepted result requires claims.".to_string(),
            );
        }
        if validity != ResultValidity::Accepted
            && let Some(fact_id) = self.active_fact_citing_result(&task_id, &result_id)
        {
            return Err(format!(
                "TaskSpace result `{result_id}` cannot be marked {} while active fact `{fact_id}` cites it. Remove or replace the fact before downgrading the result.",
                validity.as_str()
            ));
        }
        let changed_artifacts = normalize_string_vec(changed_artifacts);
        let validator_refs = normalize_string_vec(validator_refs);
        if validity == ResultValidity::Accepted {
            self.validate_accepted_result_evidence_by_node_kind(
                &map_id,
                &result_id,
                &claims,
                &evidence_refs,
                &changed_artifacts,
                &validator_refs,
            )?;
        }

        let map = self
            .maps
            .get_mut(&map_id)
            .expect("active map was validated before result validity update");
        let task_id = map.task_id.clone();
        let result = map.results.get_mut(&result_id).ok_or_else(|| {
            format!("TaskSpace result `{result_id}` does not exist on active task path `{map_id}`.")
        })?;
        result.evidence_package = NodeResultEvidencePackage {
            claims,
            evidence_refs,
            changed_artifacts,
            validator_refs,
            remaining_uncertainty: normalize_string_vec(remaining_uncertainty),
            validity,
            validity_reason: validity_reason.clone(),
        };
        Ok(vec![MapRuntimeEvent::ResultValidityChanged(
            MapRuntimeResultValidityChangedEvent {
                task_id,
                map_id,
                node_id: result.node_id.clone(),
                result_id,
                validity: validity.as_str().to_string(),
            },
        )])
    }

    fn validate_completion_evidence(&self, node_id: &str) -> Result<(), String> {
        let map_id = self.active_map_id.as_ref().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        self.validate_completion_evidence_for(map_id, node_id)
    }

    fn validate_completion_evidence_for(&self, map_id: &str, node_id: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
        match node.kind {
            NodeKind::ImplementSolution => {
                if !node_has_successful_action(map, node, ActionClass::Edit) {
                    return Err(format!(
                        "TaskSpace implement_solution node `{node_id}` cannot be completed without a recorded successful edit action. Execute the edit in this node, or block the node if the edit cannot be made."
                    ));
                }
            }
            NodeKind::SmokeTest | NodeKind::RegressionTest => {
                if !node_has_success
```

## validate_completion_evidence_for

```rust
("active map was validated before result validity update");
        let task_id = map.task_id.clone();
        let result = map.results.get_mut(&result_id).ok_or_else(|| {
            format!("TaskSpace result `{result_id}` does not exist on active task path `{map_id}`.")
        })?;
        result.evidence_package = NodeResultEvidencePackage {
            claims,
            evidence_refs,
            changed_artifacts,
            validator_refs,
            remaining_uncertainty: normalize_string_vec(remaining_uncertainty),
            validity,
            validity_reason: validity_reason.clone(),
        };
        Ok(vec![MapRuntimeEvent::ResultValidityChanged(
            MapRuntimeResultValidityChangedEvent {
                task_id,
                map_id,
                node_id: result.node_id.clone(),
                result_id,
                validity: validity.as_str().to_string(),
            },
        )])
    }

    fn validate_completion_evidence(&self, node_id: &str) -> Result<(), String> {
        let map_id = self.active_map_id.as_ref().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists.".to_string()
        })?;
        self.validate_completion_evidence_for(map_id, node_id)
    }

    fn validate_completion_evidence_for(&self, map_id: &str, node_id: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(map_id)
            .ok_or_else(|| format!("TaskSpace task path `{map_id}` is missing."))?;
        let node = map
            .nodes
            .get(node_id)
            .ok_or_else(|| format!("TaskSpace node `{node_id}` does not exist."))?;
        match node.kind {
            NodeKind::ImplementSolution => {
                if !node_has_successful_action(map, node, ActionClass::Edit) {
                    return Err(format!(
                        "TaskSpace implement_solution node `{node_id}` cannot be completed without a recorded successful edit action. Execute the edit in this node, or block the node if the edit cannot be made."
                    ));
                }
            }
            NodeKind::SmokeTest | NodeKind::RegressionTest => {
                if !node_has_successful_action(map, node, ActionClass::Test)
                    && !node_has_successful_action(map, node, ActionClass::Build)
                {
                    return Err(format!(
                        "TaskSpace {} node `{node_id}` cannot be completed without a recorded successful test or build action. Run validation in this node, or block it if validation fails and create a follow-up implementation node.",
                        node.kind.as_str()
                    ));
                }
            }
            NodeKind::InspectCodeContext | NodeKind::FinalSynthesis | NodeKind::Custom => {}
        }
        Ok(())
    }

    fn active_task_context_for_cognitive_update(
        &self,
        owner_session_id: ThreadId,
    ) -> Result<(TaskId, ActionMapId), String> {
        if self.mode != MapRuntimeMode::Experiment {
            return Err("TaskSpace mode is not active.".to_string());
        }
        let task_id = self.active_task_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task exists. Use taskspace_control(action=start_task) or taskspace_control(action=route_task) before recording cognitive state."
                .to_string()
        })?;
        let task = self
            .tasks
            .get(&task_id)
            .ok_or_else(|| format!("TaskSpace task `{task_id}` does not exist."))?;
        if let Some(owner) = task.owner_session_id
            && owner != owner_session_id
        {
            return Err(format!(
                "TaskSpace task `{task_id}` is owned by another session and cannot be updated here."
            ));
        }
        let map_id = self.active_map_id.clone().ok_or_else(|| {
            "TaskSpace mode is active but no active task path exists. Use taskspace_control(action=start_task) or taskspace_control(action=route_task) before recording cognitive state."
                .to_string()
        })?;
        if task.active_map_id.as_deref() != Some(map_id.as_str())
            || !task
                .map_ids
                .iter()
                .any(|known_map_id| known_map_id == &map_id)
        {
            return Err(format!(
                "TaskSpace active task `{task_id}` is not bound to active task path `{map_id}`."
            ));
        }
        let map = self
            .maps
            .get(&map_id)
            .ok_or_else(|| format!("TaskSpace active task path `{map_id}` is missing."))?;
        if map.task_id.as_deref() != Some(task_id.as_str()) {
            return Err(format!(
                "TaskSpace active task path `{map_id}` does not belong to active task `{task_id}`."
            ));
        }
        Ok((task_id, map_id))
    }

    fn normalize_evidence_refs(
        &self,
        task_id: &str,
        map_id: Option<&str>,
        inputs: Vec<ActionMapEvidenceRefInput>,
    ) -> Result<Vec<EvidenceRef>, String> {
        inputs
            .into_iter()
            .map(|input| self.normalize_evidence_ref(task_id, map_id, input))
            .collect()
    }

    fn normalize_evidence_refs_for_result(
        &self,
        map_id: &str,
        current_result_id: &str,
        inputs: Vec<ActionMapEvidenceRefInput>,
    ) -> Result<Vec<EvidenceRef>, String> {
        let task_id = self
            .maps
            .get(map_id)
            .and_then(|map| map.task_id.as_deref())
            .unwrap_or("");
        inputs
            .into_iter()
            .map(|input| {
                self.normalize_evidence_ref_for_result_scope(
                    task_id,
                    map_id,
                    current_result_id,
                    input,
                )
            })
            .collect()
    }

    fn normalize_claim_inputs_for_result(
        &self,
        map_id: &str,
        current_result_id: &str,
        inputs: Vec<ActionMapCognitiveClaimInput>,
    ) -> Result<Vec<CognitiveClaim>, String> {
        let task_id = self
            .maps
            .get(map_id)

```
