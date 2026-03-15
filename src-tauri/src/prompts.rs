#[derive(Debug, Clone)]
pub struct RoundContent {
    pub speaker: String,
    pub content: String,
    pub round_number: i32,
}

pub fn build_arena_system_prompt(
    role: &str,
    topic: &str,
    round: i32,
    word_limit: u32,
    history: &[RoundContent],
    speaker: &str,
) -> String {
    let position = match role {
        "pro" => "IN FAVOR OF",
        "con" => "AGAINST",
        _ => "ON",
    };

    let mut prompt = format!("You are arguing {position} the following topic: {topic}\n\n");

    if round == 1 {
        prompt.push_str(
            "This is the opening round. Establish your position clearly with your strongest arguments.\n\n",
        );
    } else {
        // Find opponent's last message
        let opponent_last = history
            .iter()
            .filter(|r| r.speaker != speaker)
            .max_by_key(|r| r.round_number);

        if let Some(opponent) = opponent_last {
            prompt.push_str(&format!(
                "Your opponent just argued:\n\n\"{}\"\n\nRespond directly to their specific points. Quote their words when rebutting.\n\n",
                opponent.content
            ));
        }
    }

    prompt.push_str(
        "Rules:\n\
         - Be persuasive but respectful.\n\
         - Use evidence and logic.\n\
         - Never concede without pivoting to a stronger counterargument.\n\
         - Do NOT repeat your previous arguments.\n\n",
    );

    prompt.push_str(&format!("Keep your response under {word_limit} words."));

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_round1_prompt() {
        let prompt = build_arena_system_prompt("pro", "AI is beneficial", 1, 300, &[], "model_a");
        assert!(prompt.contains("IN FAVOR OF"), "should contain IN FAVOR OF");
        assert!(
            prompt.contains("Establish your position"),
            "round 1 should contain opening instruction"
        );
    }

    #[test]
    fn con_round1_prompt() {
        let prompt = build_arena_system_prompt("con", "AI is beneficial", 1, 300, &[], "model_b");
        assert!(prompt.contains("AGAINST"), "should contain AGAINST");
    }

    #[test]
    fn round2_includes_opponent() {
        let history = vec![
            RoundContent {
                speaker: "model_a".into(),
                content: "AI helps cure diseases and accelerates research.".into(),
                round_number: 1,
            },
            RoundContent {
                speaker: "model_b".into(),
                content: "AI poses existential risk to humanity.".into(),
                round_number: 1,
            },
        ];

        let prompt =
            build_arena_system_prompt("pro", "AI is beneficial", 2, 300, &history, "model_a");
        assert!(
            prompt.contains("AI poses existential risk"),
            "should include opponent's text"
        );
        assert!(
            prompt.contains("Respond directly"),
            "should instruct to respond"
        );
    }

    #[test]
    fn word_limit_in_prompt() {
        let prompt = build_arena_system_prompt("pro", "test topic", 1, 300, &[], "model_a");
        assert!(prompt.contains("300 words"), "should contain word limit");
    }
}
