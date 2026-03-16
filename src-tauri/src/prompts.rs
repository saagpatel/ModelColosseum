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

pub fn build_sparring_system_prompt(
    difficulty: &str,
    role: &str,
    topic: &str,
    phase: &str,
    word_limit: u32,
    history: &[RoundContent],
) -> String {
    let position = match role {
        "pro" => "IN FAVOR OF",
        "con" => "AGAINST",
        _ => "ON",
    };

    // Persona by difficulty
    let persona = match difficulty {
        "casual" => format!(
            "You are debating a human. You are arguing {position}: {topic}. \
             Be challenging but fair. If they make a good point, acknowledge it briefly before countering."
        ),
        "competitive" => format!(
            "You are a skilled debater arguing {position}: {topic}. \
             Directly quote your opponent's words and explain why they're wrong. \
             Exploit logical weaknesses. Never concede."
        ),
        "expert" => format!(
            "You are a domain expert debating {position}: {topic}. \
             Bring specific data, historical examples, and expert analysis. \
             Ask pointed rhetorical questions. Challenge every assumption."
        ),
        _ => format!("You are debating {position}: {topic}."),
    };

    let mut prompt = persona;
    prompt.push_str("\n\n");

    // Phase instructions
    let phase_instruction = match phase {
        "opening" => "Establish your position with your strongest arguments.",
        "rebuttal" => "Counter your opponent's specific points. Quote their words when disagreeing.",
        "closing" => "Synthesize your strongest arguments. Address the most compelling counter-points raised.",
        _ => "Continue the debate.",
    };
    prompt.push_str(phase_instruction);
    prompt.push_str("\n\n");

    // Transcript
    if !history.is_empty() {
        prompt.push_str("Debate transcript so far:\n\n");
        for entry in history {
            let speaker_label = match entry.speaker.as_str() {
                "human" => "HUMAN",
                "model_a" => "AI",
                _ => &entry.speaker,
            };
            prompt.push_str(&format!(
                "[Round {} - {}]: {}\n\n",
                entry.round_number, speaker_label, entry.content
            ));
        }
    }

    prompt.push_str(&format!("Keep your response under {} words.", word_limit));

    prompt
}

pub fn build_scorecard_judge_prompt(
    topic: &str,
    human_side: &str,
    history: &[RoundContent],
) -> String {
    let human_position = match human_side {
        "pro" => "IN FAVOR OF",
        "con" => "AGAINST",
        _ => "ON",
    };
    let ai_position = match human_side {
        "pro" => "AGAINST",
        "con" => "IN FAVOR OF",
        _ => "ON",
    };

    let mut prompt = format!(
        "You are an impartial debate judge. Evaluate the following debate on the topic: \"{topic}\"\n\n\
         The HUMAN argued {human_position} the topic.\n\
         The AI argued {ai_position} the topic.\n\n"
    );

    if !history.is_empty() {
        prompt.push_str("## Full Debate Transcript\n\n");
        for entry in history {
            let speaker_label = match entry.speaker.as_str() {
                "human" => "HUMAN",
                "model_a" => "AI",
                _ => &entry.speaker,
            };
            prompt.push_str(&format!(
                "[Round {} - {}]: {}\n\n",
                entry.round_number, speaker_label, entry.content
            ));
        }
    }

    prompt.push_str(
        "## Scoring Instructions\n\n\
         Score each debater on four dimensions (1 = poor, 10 = excellent):\n\
         - persuasiveness: How convincing and rhetorically effective were their arguments?\n\
         - evidence: Did they use facts, examples, and logical reasoning effectively?\n\
         - coherence: Were their arguments well-structured and internally consistent?\n\
         - rebuttal: How effectively did they counter the opponent's specific points?\n\n\
         Also provide:\n\
         - strongest_human_point: The single most compelling argument the human made.\n\
         - weakest_human_point: The human's least convincing argument or biggest logical gap.\n\
         - missed_argument: An important argument the human failed to make.\n\
         - improvement_tip: One concrete, actionable tip to help the human debate better.\n\n\
         Output ONLY valid JSON with this exact structure, no markdown, no explanation:\n\
         {\n\
           \"human\": { \"persuasiveness\": 1-10, \"evidence\": 1-10, \"coherence\": 1-10, \"rebuttal\": 1-10 },\n\
           \"ai\": { \"persuasiveness\": 1-10, \"evidence\": 1-10, \"coherence\": 1-10, \"rebuttal\": 1-10 },\n\
           \"strongest_human_point\": \"...\",\n\
           \"weakest_human_point\": \"...\",\n\
           \"missed_argument\": \"...\",\n\
           \"improvement_tip\": \"...\"\n\
         }"
    );

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

    #[test]
    fn casual_opening_prompt() {
        let prompt =
            build_sparring_system_prompt("casual", "pro", "AI is beneficial", "opening", 200, &[]);
        assert!(
            prompt.contains("challenging but fair"),
            "casual should be fair"
        );
        assert!(prompt.contains("IN FAVOR OF"), "should contain position");
        assert!(
            prompt.contains("Establish your position"),
            "should contain opening instruction"
        );
        assert!(prompt.contains("200 words"), "should contain word limit");
    }

    #[test]
    fn competitive_rebuttal_prompt() {
        let history = vec![
            RoundContent {
                speaker: "human".into(),
                content: "AI creates jobs.".into(),
                round_number: 1,
            },
            RoundContent {
                speaker: "model_a".into(),
                content: "AI destroys more jobs.".into(),
                round_number: 2,
            },
        ];
        let prompt = build_sparring_system_prompt(
            "competitive",
            "con",
            "AI is beneficial",
            "rebuttal",
            300,
            &history,
        );
        assert!(
            prompt.contains("Exploit logical weaknesses"),
            "competitive should be aggressive"
        );
        assert!(
            prompt.contains("Counter your opponent"),
            "should contain rebuttal instruction"
        );
        assert!(
            prompt.contains("[Round 1 - HUMAN]"),
            "should label human rounds"
        );
        assert!(prompt.contains("[Round 2 - AI]"), "should label AI rounds");
    }

    #[test]
    fn expert_closing_prompt() {
        let prompt =
            build_sparring_system_prompt("expert", "pro", "Climate change", "closing", 150, &[]);
        assert!(
            prompt.contains("domain expert"),
            "expert should mention expertise"
        );
        assert!(
            prompt.contains("Synthesize your strongest"),
            "should contain closing instruction"
        );
        assert!(prompt.contains("150 words"), "should contain word limit");
    }

    #[test]
    fn con_role_prompt() {
        let prompt =
            build_sparring_system_prompt("casual", "con", "Free will exists", "opening", 200, &[]);
        assert!(prompt.contains("AGAINST"), "con should contain AGAINST");
    }

    #[test]
    fn empty_history_no_transcript_section() {
        let prompt = build_sparring_system_prompt("casual", "pro", "Test", "opening", 200, &[]);
        assert!(
            !prompt.contains("Debate transcript"),
            "empty history should not have transcript section"
        );
    }

    #[test]
    fn scorecard_judge_prompt_contains_transcript() {
        let history = vec![
            RoundContent {
                speaker: "human".into(),
                content: "AI creates more jobs than it destroys.".into(),
                round_number: 1,
            },
            RoundContent {
                speaker: "model_a".into(),
                content: "Historical evidence shows automation displaces workers.".into(),
                round_number: 2,
            },
        ];
        let prompt = build_scorecard_judge_prompt("AI and employment", "pro", &history);
        assert!(
            prompt.contains("[Round 1 - HUMAN]"),
            "should label human round"
        );
        assert!(prompt.contains("[Round 2 - AI]"), "should label AI round");
        assert!(
            prompt.contains("AI creates more jobs"),
            "should include human content"
        );
        assert!(
            prompt.contains("Historical evidence"),
            "should include AI content"
        );
        assert!(
            prompt.contains("IN FAVOR OF"),
            "human side should be IN FAVOR OF for pro"
        );
    }

    #[test]
    fn scorecard_judge_prompt_contains_json_template() {
        let prompt = build_scorecard_judge_prompt("Free will", "con", &[]);
        assert!(
            prompt.contains("\"persuasiveness\""),
            "should contain persuasiveness field"
        );
        assert!(
            prompt.contains("\"evidence\""),
            "should contain evidence field"
        );
        assert!(
            prompt.contains("\"coherence\""),
            "should contain coherence field"
        );
        assert!(
            prompt.contains("\"rebuttal\""),
            "should contain rebuttal field"
        );
        assert!(
            prompt.contains("\"strongest_human_point\""),
            "should contain strongest_human_point"
        );
        assert!(
            prompt.contains("\"improvement_tip\""),
            "should contain improvement_tip"
        );
        assert!(
            prompt.contains("Output ONLY valid JSON"),
            "should instruct JSON-only output"
        );
        assert!(
            prompt.contains("AGAINST"),
            "con human_side should map to AGAINST"
        );
    }
}
