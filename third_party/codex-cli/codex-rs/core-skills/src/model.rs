use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use codex_exec_server::ExecutorFileSystem;
use codex_protocol::protocol::Product;
use codex_protocol::protocol::SkillScope;
use codex_utils_absolute_path::AbsolutePathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub interface: Option<SkillInterface>,
    pub dependencies: Option<SkillDependencies>,
    pub policy: Option<SkillPolicy>,
    /// Path to the SKILLS.md file that declares this skill.
    pub path_to_skills_md: AbsolutePathBuf,
    pub scope: SkillScope,
}

impl SkillMetadata {
    fn allow_implicit_invocation(&self) -> bool {
        self.policy
            .as_ref()
            .and_then(|policy| policy.allow_implicit_invocation)
            .unwrap_or(true)
    }

    pub fn matches_product_restriction_for_product(
        &self,
        restriction_product: Option<Product>,
    ) -> bool {
        match &self.policy {
            Some(policy) => {
                policy.products.is_empty()
                    || restriction_product.is_some_and(|product| {
                        product.matches_product_restriction(&policy.products)
                    })
            }
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillPolicy {
    pub allow_implicit_invocation: Option<bool>,
    // TODO: Enforce product gating in Codex skill selection/injection instead of only parsing and
    // storing this metadata.
    pub products: Vec<Product>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<AbsolutePathBuf>,
    pub icon_large: Option<AbsolutePathBuf>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDependencies {
    pub tools: Vec<SkillToolDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillToolDependency {
    pub r#type: String,
    pub value: String,
    pub description: Option<String>,
    pub transport: Option<String>,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillError {
    pub path: AbsolutePathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct SkillLoadOutcome {
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillError>,
    pub disabled_paths: HashSet<AbsolutePathBuf>,
    pub(crate) skill_roots: Vec<AbsolutePathBuf>,
    pub(crate) skill_root_by_path: Arc<HashMap<AbsolutePathBuf, AbsolutePathBuf>>,
    pub(crate) file_systems_by_skill_path: SkillFileSystemsByPath,
    pub(crate) implicit_skills_by_scripts_dir: Arc<HashMap<AbsolutePathBuf, SkillMetadata>>,
    pub(crate) implicit_skills_by_doc_path: Arc<HashMap<AbsolutePathBuf, SkillMetadata>>,
    pub(crate) integrity_by_path: Arc<HashMap<AbsolutePathBuf, String>>,
}

impl SkillLoadOutcome {
    pub fn is_skill_enabled(&self, skill: &SkillMetadata) -> bool {
        !self.disabled_paths.contains(&skill.path_to_skills_md)
    }

    pub fn is_skill_allowed_for_implicit_invocation(&self, skill: &SkillMetadata) -> bool {
        self.is_skill_enabled(skill) && skill.allow_implicit_invocation()
    }

    pub fn allowed_skills_for_implicit_invocation(&self) -> Vec<SkillMetadata> {
        self.skills
            .iter()
            .filter(|skill| self.is_skill_allowed_for_implicit_invocation(skill))
            .cloned()
            .collect()
    }

    pub fn skills_with_enabled(&self) -> impl Iterator<Item = (&SkillMetadata, bool)> {
        self.skills
            .iter()
            .map(|skill| (skill, self.is_skill_enabled(skill)))
    }

    pub(crate) fn file_system_for_skill(
        &self,
        skill: &SkillMetadata,
    ) -> Option<Arc<dyn ExecutorFileSystem>> {
        self.file_systems_by_skill_path
            .get(&skill.path_to_skills_md)
    }

    pub fn expected_body_sha256(&self, skill: &SkillMetadata) -> Option<&str> {
        self.integrity_by_path
            .get(&skill.path_to_skills_md)
            .map(String::as_str)
    }

    pub fn remove_skill_at_path(&mut self, path: &AbsolutePathBuf) -> Option<SkillMetadata> {
        let index = self
            .skills
            .iter()
            .position(|skill| &skill.path_to_skills_md == path)?;
        let removed = self.skills.remove(index);
        self.disabled_paths.remove(path);
        Arc::make_mut(&mut self.skill_root_by_path).remove(path);
        self.file_systems_by_skill_path.remove(path);
        Arc::make_mut(&mut self.implicit_skills_by_doc_path)
            .retain(|_, skill| &skill.path_to_skills_md != path);
        Arc::make_mut(&mut self.implicit_skills_by_scripts_dir)
            .retain(|_, skill| &skill.path_to_skills_md != path);
        Arc::make_mut(&mut self.integrity_by_path).remove(path);
        Some(removed)
    }

    pub fn rebind_skill_to_snapshot(
        &mut self,
        source_path: &AbsolutePathBuf,
        snapshot_path: AbsolutePathBuf,
        body_sha256: String,
    ) -> Result<(), String> {
        let Some(index) = self
            .skills
            .iter()
            .position(|skill| &skill.path_to_skills_md == source_path)
        else {
            return Err(format!(
                "skill source path is not loaded: {}",
                source_path.display()
            ));
        };
        if self
            .skills
            .iter()
            .any(|skill| skill.path_to_skills_md == snapshot_path)
        {
            return Err(format!(
                "skill snapshot path is already loaded: {}",
                snapshot_path.display()
            ));
        }

        let mut rebound = self.skills[index].clone();
        rebound.path_to_skills_md = snapshot_path.clone();
        self.skills[index] = rebound.clone();

        let source_disabled = self.disabled_paths.remove(source_path);
        if source_disabled {
            self.disabled_paths.insert(snapshot_path.clone());
        }
        if let Some(root) = Arc::make_mut(&mut self.skill_root_by_path).remove(source_path) {
            Arc::make_mut(&mut self.skill_root_by_path).insert(snapshot_path.clone(), root);
        }
        self.file_systems_by_skill_path
            .rebind(source_path, &snapshot_path);
        rebind_implicit_skill_index(&mut self.implicit_skills_by_doc_path, source_path, &rebound);
        rebind_implicit_skill_index(
            &mut self.implicit_skills_by_scripts_dir,
            source_path,
            &rebound,
        );
        let integrity = Arc::make_mut(&mut self.integrity_by_path);
        integrity.remove(source_path);
        integrity.insert(snapshot_path, body_sha256);
        Ok(())
    }
}

fn rebind_implicit_skill_index(
    index: &mut Arc<HashMap<AbsolutePathBuf, SkillMetadata>>,
    source_path: &AbsolutePathBuf,
    rebound: &SkillMetadata,
) {
    for skill in Arc::make_mut(index).values_mut() {
        if &skill.path_to_skills_md == source_path {
            *skill = rebound.clone();
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct SkillFileSystemsByPath {
    values: Arc<HashMap<AbsolutePathBuf, Arc<dyn ExecutorFileSystem>>>,
}

impl SkillFileSystemsByPath {
    pub(crate) fn new(values: HashMap<AbsolutePathBuf, Arc<dyn ExecutorFileSystem>>) -> Self {
        Self {
            values: Arc::new(values),
        }
    }

    fn get(&self, path: &AbsolutePathBuf) -> Option<Arc<dyn ExecutorFileSystem>> {
        self.values.get(path).map(Arc::clone)
    }

    fn retain_paths(&mut self, paths: &HashSet<AbsolutePathBuf>) {
        self.values = Arc::new(
            self.values
                .iter()
                .filter(|(path, _)| paths.contains(*path))
                .map(|(path, fs)| (path.clone(), Arc::clone(fs)))
                .collect(),
        );
    }

    fn remove(&mut self, path: &AbsolutePathBuf) {
        Arc::make_mut(&mut self.values).remove(path);
    }

    fn rebind(&mut self, source_path: &AbsolutePathBuf, snapshot_path: &AbsolutePathBuf) {
        let values = Arc::make_mut(&mut self.values);
        if let Some(file_system) = values.remove(source_path) {
            values.insert(snapshot_path.clone(), file_system);
        }
    }
}

impl fmt::Debug for SkillFileSystemsByPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SkillFileSystemsByPath")
            .field("len", &self.values.len())
            .finish()
    }
}

pub fn filter_skill_load_outcome_for_product(
    mut outcome: SkillLoadOutcome,
    restriction_product: Option<Product>,
) -> SkillLoadOutcome {
    outcome
        .skills
        .retain(|skill| skill.matches_product_restriction_for_product(restriction_product));
    let retained_paths: HashSet<AbsolutePathBuf> = outcome
        .skills
        .iter()
        .map(|skill| skill.path_to_skills_md.clone())
        .collect();
    outcome
        .file_systems_by_skill_path
        .retain_paths(&retained_paths);
    outcome.skill_root_by_path = Arc::new(
        outcome
            .skill_root_by_path
            .iter()
            .filter(|(path, _)| retained_paths.contains(*path))
            .map(|(path, root)| (path.clone(), root.clone()))
            .collect(),
    );
    let retained_roots: HashSet<AbsolutePathBuf> =
        outcome.skill_root_by_path.values().cloned().collect();
    outcome
        .skill_roots
        .retain(|root| retained_roots.contains(root));
    outcome.implicit_skills_by_scripts_dir = Arc::new(
        outcome
            .implicit_skills_by_scripts_dir
            .iter()
            .filter(|(_, skill)| skill.matches_product_restriction_for_product(restriction_product))
            .map(|(path, skill)| (path.clone(), skill.clone()))
            .collect(),
    );
    outcome.implicit_skills_by_doc_path = Arc::new(
        outcome
            .implicit_skills_by_doc_path
            .iter()
            .filter(|(_, skill)| skill.matches_product_restriction_for_product(restriction_product))
            .map(|(path, skill)| (path.clone(), skill.clone()))
            .collect(),
    );
    outcome.integrity_by_path = Arc::new(
        outcome
            .integrity_by_path
            .iter()
            .filter(|(path, _)| retained_paths.contains(*path))
            .map(|(path, hash)| (path.clone(), hash.clone()))
            .collect(),
    );
    outcome
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use codex_protocol::protocol::SkillScope;
    use codex_utils_absolute_path::test_support::PathBufExt;
    use codex_utils_absolute_path::test_support::test_path_buf;

    fn skill(path: &str) -> SkillMetadata {
        SkillMetadata {
            name: "taskspace-advanced".to_string(),
            description: "advanced TaskSpace guidance".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf(path).abs(),
            scope: SkillScope::System,
        }
    }

    #[test]
    fn rebind_skill_to_snapshot_moves_catalog_identity_and_integrity() {
        let source = test_path_buf("/tmp/.system/taskspace-advanced/SKILL.md").abs();
        let snapshot =
            test_path_buf("/tmp/.system/.snapshots/abc/taskspace-advanced/SKILL.md").abs();
        let mut outcome = SkillLoadOutcome {
            skills: vec![skill(source.to_string_lossy().as_ref())],
            ..Default::default()
        };

        outcome
            .rebind_skill_to_snapshot(&source, snapshot.clone(), "abc".to_string())
            .expect("rebind snapshot");

        assert_eq!(outcome.skills[0].path_to_skills_md, snapshot);
        assert_eq!(
            outcome.expected_body_sha256(&outcome.skills[0]),
            Some("abc")
        );
        assert!(outcome.remove_skill_at_path(&snapshot).is_some());
        assert!(outcome.skills.is_empty());
    }
}
