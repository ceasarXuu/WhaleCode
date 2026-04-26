use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionRoute {
    LocalReply { message: String },
    AgentTask,
}

pub fn route_interaction(input: &str, cwd: &Path) -> InteractionRoute {
    if is_casual_input(input) {
        return InteractionRoute::LocalReply {
            message: format!(
                "Hi. Workspace: {}. Tell me what code task you want me to inspect, change, or debug.",
                cwd.display()
            ),
        };
    }
    InteractionRoute::AgentTask
}

fn is_casual_input(input: &str) -> bool {
    let normalized = normalize(input);
    matches!(
        normalized.as_str(),
        "hi" | "hello"
            | "hey"
            | "hi there"
            | "hello there"
            | "hey there"
            | "thanks"
            | "thank you"
            | "thx"
            | "ok"
            | "okay"
            | "你好"
            | "您好"
            | "嗨"
            | "哈喽"
            | "谢谢"
    )
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .trim_matches(|ch: char| {
            ch.is_ascii_punctuation()
                || matches!(ch, '。' | '，' | '！' | '？' | '、' | '；' | '：')
        })
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{route_interaction, InteractionRoute};

    #[test]
    fn routes_pure_greetings_to_local_reply() {
        let route = route_interaction("hi", Path::new("/repo"));

        assert_eq!(
            route,
            InteractionRoute::LocalReply {
                message: "Hi. Workspace: /repo. Tell me what code task you want me to inspect, change, or debug.".to_owned()
            }
        );
    }

    #[test]
    fn keeps_actionable_prompts_in_agent_loop() {
        assert_eq!(
            route_interaction("hi, inspect this repository", Path::new("/repo")),
            InteractionRoute::AgentTask
        );
        assert_eq!(
            route_interaction("fix the failing test", Path::new("/repo")),
            InteractionRoute::AgentTask
        );
    }
}
