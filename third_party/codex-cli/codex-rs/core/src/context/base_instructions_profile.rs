#[cfg(test)]
use codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_STANDARD;
use codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_TASKSPACE;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::MapRuntimeMode;
use sha2::Digest;
use sha2::Sha256;

pub(crate) const WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION: &str = "1.0.0";
pub(crate) const WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256: &str =
    "7c27bcb65c43dcfc9eb38284e6968003f2cb3bbaff3568437740208e39cadf19";
pub(crate) const WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION: &str = "1.0.0";
pub(crate) const WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256: &str =
    "95f6cc4eac04af52fd052e23f63e5ba2ccffcca59b041b68ff4269f6b78d8d55";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WhaleCodeBaseInstructionsProfile {
    Standard,
    TaskSpace,
}

impl WhaleCodeBaseInstructionsProfile {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::TaskSpace => "taskspace",
        }
    }

    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Standard => WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION,
            Self::TaskSpace => WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_VERSION,
        }
    }

    pub(crate) fn expected_sha256(self) -> &'static str {
        match self {
            Self::Standard => WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256,
            Self::TaskSpace => WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256,
        }
    }

    pub(crate) fn is_taskspace(self) -> bool {
        self == Self::TaskSpace
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedBaseInstructions {
    pub(crate) instructions: BaseInstructions,
    pub(crate) profile: WhaleCodeBaseInstructionsProfile,
    pub(crate) version: &'static str,
    pub(crate) sha256: String,
    pub(crate) bytes: usize,
    pub(crate) matches_current_contract: bool,
}

pub(crate) fn resolve_base_instructions(
    standard_instructions: &str,
    mode: MapRuntimeMode,
) -> ResolvedBaseInstructions {
    let profile = match mode {
        MapRuntimeMode::Standard => WhaleCodeBaseInstructionsProfile::Standard,
        MapRuntimeMode::Experiment => WhaleCodeBaseInstructionsProfile::TaskSpace,
    };
    let text = match profile {
        WhaleCodeBaseInstructionsProfile::Standard => standard_instructions,
        WhaleCodeBaseInstructionsProfile::TaskSpace => BASE_INSTRUCTIONS_WHALECODE_TASKSPACE,
    };
    let sha256 = sha256(text);
    let matches_current_contract = sha256 == profile.expected_sha256();
    ResolvedBaseInstructions {
        instructions: BaseInstructions {
            text: text.to_string(),
        },
        profile,
        version: if matches_current_contract {
            profile.version()
        } else {
            "custom"
        },
        bytes: text.len(),
        matches_current_contract,
        sha256,
    }
}

fn sha256(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_hashes_match_versioned_contracts() {
        assert_eq!(
            sha256(BASE_INSTRUCTIONS_WHALECODE_STANDARD),
            WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256
        );
        assert_eq!(
            sha256(BASE_INSTRUCTIONS_WHALECODE_TASKSPACE),
            WHALECODE_TASKSPACE_BASE_INSTRUCTIONS_SHA256
        );
    }

    #[test]
    fn runtime_mode_selects_one_complete_base_prompt() {
        let standard = resolve_base_instructions(
            BASE_INSTRUCTIONS_WHALECODE_STANDARD,
            MapRuntimeMode::Standard,
        );
        let taskspace = resolve_base_instructions(
            BASE_INSTRUCTIONS_WHALECODE_STANDARD,
            MapRuntimeMode::Experiment,
        );

        assert_eq!(standard.profile, WhaleCodeBaseInstructionsProfile::Standard);
        assert_eq!(
            standard.instructions.text,
            BASE_INSTRUCTIONS_WHALECODE_STANDARD
        );
        assert!(standard.matches_current_contract);
        assert_eq!(
            taskspace.profile,
            WhaleCodeBaseInstructionsProfile::TaskSpace
        );
        assert_eq!(
            taskspace.instructions.text,
            BASE_INSTRUCTIONS_WHALECODE_TASKSPACE
        );
        assert!(taskspace.matches_current_contract);
    }

    #[test]
    fn standard_override_remains_visible_and_is_marked_non_contract() {
        let resolved = resolve_base_instructions("custom standard", MapRuntimeMode::Standard);

        assert_eq!(resolved.instructions.text, "custom standard");
        assert_eq!(resolved.version, "custom");
        assert!(!resolved.matches_current_contract);
    }
}
