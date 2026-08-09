#[cfg(test)]
use codex_protocol::models::BASE_INSTRUCTIONS_WHALECODE_STANDARD;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::MapRuntimeMode;
use sha2::Digest;
use sha2::Sha256;

pub(crate) const WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION: &str = "1.0.2";
pub(crate) const WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256: &str =
    "5e1178bd781d3be2cb2c4d5ead76ba074b3349954b7832333d86b6c454cc7382";

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
            Self::TaskSpace => WHALECODE_STANDARD_BASE_INSTRUCTIONS_VERSION,
        }
    }

    pub(crate) fn expected_sha256(self) -> &'static str {
        match self {
            Self::Standard => WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256,
            Self::TaskSpace => WHALECODE_STANDARD_BASE_INSTRUCTIONS_SHA256,
        }
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
        WhaleCodeBaseInstructionsProfile::TaskSpace => standard_instructions,
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
    }

    #[test]
    fn whalecode_base_prompts_do_not_embed_tool_wire_examples() {
        const FORBIDDEN_TOOL_WIRE_FRAGMENTS: &[&str] = &[
            "*** Begin Patch",
            "*** Update File:",
            "taskspace_exec",
            "initialize_map",
            "hosted_bindings",
            "{\"command\"",
            "{\"input\"",
            "\"arguments\"",
        ];

        for (profile, prompt) in [("standard", BASE_INSTRUCTIONS_WHALECODE_STANDARD)] {
            for fragment in FORBIDDEN_TOOL_WIRE_FRAGMENTS {
                assert!(
                    !prompt.contains(fragment),
                    "{profile} Base embeds Tool wire syntax: {fragment}"
                );
            }
        }
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
            BASE_INSTRUCTIONS_WHALECODE_STANDARD
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
