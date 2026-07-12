use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MIN_CONFIDENCE_SAMPLES: usize = 3;
const MIN_SCORE: f64 = 1.0;
const MAX_SCORE: f64 = 10.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EvaluationConfig {
    pub repetitions: u32,
    pub warmup_repetitions: u32,
    pub timeout_seconds: u64,
    pub temperature: f64,
    pub num_predict: Option<u32>,
    pub think: bool,
    pub seed: Option<u64>,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            repetitions: 3,
            warmup_repetitions: 1,
            timeout_seconds: 120,
            temperature: 0.2,
            num_predict: Some(1024),
            think: false,
            seed: None,
        }
    }
}

impl EvaluationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=20).contains(&self.repetitions) {
            return Err("repetitions must be between 1 and 20".into());
        }
        if self.warmup_repetitions > 3 {
            return Err("warmup_repetitions must be between 0 and 3".into());
        }
        if !(5..=3_600).contains(&self.timeout_seconds) {
            return Err("timeout_seconds must be between 5 and 3600".into());
        }
        if !(0.0..=2.0).contains(&self.temperature) || !self.temperature.is_finite() {
            return Err("temperature must be a finite value between 0 and 2".into());
        }
        if let Some(num_predict) = self.num_predict {
            if !(1..=32_768).contains(&num_predict) {
                return Err("num_predict must be between 1 and 32768".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSnapshot {
    pub id: i64,
    pub category: String,
    pub title: String,
    pub text: String,
    pub system_prompt: Option<String>,
    pub ideal_answer: Option<String>,
    pub eval_criteria: Option<String>,
    pub sort_order: i64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteSnapshot {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub digest: String,
    pub prompts: Vec<PromptSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSnapshot {
    pub database_id: i64,
    pub exact_tag: String,
    pub digest: Option<String>,
    pub size_bytes: Option<u64>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub family: Option<String>,
    pub modified_at: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSnapshot {
    pub server_version: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSnapshot {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: String,
    pub cpu_brand: Option<String>,
    pub logical_cpu_count: usize,
    pub total_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub run_key: String,
    pub created_at_unix_ms: u64,
    pub suite: SuiteSnapshot,
    pub models: Vec<ModelSnapshot>,
    pub ollama: OllamaSnapshot,
    pub hardware: HardwareSnapshot,
    pub generation: EvaluationConfig,
    pub measured_trial_count: usize,
    pub warmup_trial_count: usize,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn digest_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|e| format!("digest serialization failed: {e}"))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrialKind {
    Warmup,
    Measured,
}

impl TrialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warmup => "warmup",
            Self::Measured => "measured",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrialPlanItem {
    pub prompt_id: i64,
    pub model_id: i64,
    pub repetition_index: u32,
    pub kind: TrialKind,
    pub execution_order: usize,
    pub generation_seed: u64,
    pub comparison_position: Option<String>,
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let swap_with = (self.next() as usize) % (index + 1);
            values.swap(index, swap_with);
        }
    }
}

pub fn balanced_positions(count: usize, seed: u64) -> Vec<bool> {
    let mut positions = Vec::with_capacity(count);
    let left_count = count.div_ceil(2);
    positions.extend(std::iter::repeat_n(true, left_count));
    positions.extend(std::iter::repeat_n(false, count - left_count));
    SplitMix64::new(seed ^ 0xa076_1d64_78bd_642f).shuffle(&mut positions);
    positions
}

pub fn build_trial_plan(
    prompt_ids: &[i64],
    model_ids: &[i64],
    config: &EvaluationConfig,
    seed: u64,
) -> Result<Vec<TrialPlanItem>, String> {
    config.validate()?;
    if prompt_ids.is_empty() {
        return Err("cannot build trials without prompts".into());
    }
    if model_ids.is_empty() {
        return Err("cannot build trials without models".into());
    }

    let mut rng = SplitMix64::new(seed);
    let mut plan = Vec::new();

    let mut warmups = Vec::new();
    for repetition_index in 0..config.warmup_repetitions {
        for model_id in model_ids {
            warmups.push(TrialPlanItem {
                prompt_id: prompt_ids[0],
                model_id: *model_id,
                repetition_index,
                kind: TrialKind::Warmup,
                execution_order: 0,
                generation_seed: rng.next() & i64::MAX as u64,
                comparison_position: None,
            });
        }
    }
    rng.shuffle(&mut warmups);
    plan.extend(warmups);

    let pair_count = if model_ids.len() == 2 {
        prompt_ids.len() * config.repetitions as usize
    } else {
        0
    };
    let positions = balanced_positions(pair_count, seed);
    let mut position_by_pair = HashMap::new();
    let mut position_index = 0usize;
    for prompt_id in prompt_ids {
        for repetition_index in 0..config.repetitions {
            if model_ids.len() == 2 {
                position_by_pair.insert((*prompt_id, repetition_index), positions[position_index]);
                position_index += 1;
            }
        }
    }

    let mut measured = Vec::new();
    for prompt_id in prompt_ids {
        for repetition_index in 0..config.repetitions {
            for (model_index, model_id) in model_ids.iter().enumerate() {
                let comparison_position = position_by_pair
                    .get(&(*prompt_id, repetition_index))
                    .map(|model_a_is_left| {
                        let is_left = if model_index == 0 {
                            *model_a_is_left
                        } else {
                            !*model_a_is_left
                        };
                        if is_left { "left" } else { "right" }.to_string()
                    });
                measured.push(TrialPlanItem {
                    prompt_id: *prompt_id,
                    model_id: *model_id,
                    repetition_index,
                    kind: TrialKind::Measured,
                    execution_order: 0,
                    generation_seed: rng.next() & i64::MAX as u64,
                    comparison_position,
                });
            }
        }
    }
    rng.shuffle(&mut measured);
    plan.extend(measured);

    for (execution_order, item) in plan.iter_mut().enumerate() {
        item.execution_order = execution_order;
    }
    Ok(plan)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceSummary {
    pub sample_size: usize,
    pub mean: Option<f64>,
    pub lower_95: Option<f64>,
    pub upper_95: Option<f64>,
    pub sufficient_sample: bool,
    pub warning: Option<String>,
}

pub fn mean_confidence_95(values: &[f64]) -> ConfidenceSummary {
    let sample_size = values.len();
    if sample_size == 0 {
        return ConfidenceSummary {
            sample_size,
            mean: None,
            lower_95: None,
            upper_95: None,
            sufficient_sample: false,
            warning: Some("No valid measured samples".into()),
        };
    }

    let mean = values.iter().sum::<f64>() / sample_size as f64;
    if sample_size == 1 {
        return ConfidenceSummary {
            sample_size,
            mean: Some(mean),
            lower_95: None,
            upper_95: None,
            sufficient_sample: false,
            warning: Some("One sample cannot estimate uncertainty".into()),
        };
    }

    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (sample_size - 1) as f64;
    let margin = 1.96 * (variance / sample_size as f64).sqrt();
    let sufficient_sample = sample_size >= MIN_CONFIDENCE_SAMPLES;
    ConfidenceSummary {
        sample_size,
        mean: Some(mean),
        lower_95: Some((mean - margin).max(MIN_SCORE)),
        upper_95: Some((mean + margin).min(MAX_SCORE)),
        sufficient_sample,
        warning: (!sufficient_sample).then(|| {
            format!(
                "Only {sample_size} samples; at least {MIN_CONFIDENCE_SAMPLES} are required for a recommendation"
            )
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideOutcome {
    Left,
    Right,
    Tie,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionBiasSummary {
    pub sample_size: usize,
    pub left_preference_rate: Option<f64>,
    pub detected: bool,
    pub warning: Option<String>,
}

pub fn detect_position_bias(outcomes: &[SideOutcome]) -> PositionBiasSummary {
    if outcomes.is_empty() {
        return PositionBiasSummary {
            sample_size: 0,
            left_preference_rate: None,
            detected: false,
            warning: Some("No human comparison votes".into()),
        };
    }
    let left_points = outcomes
        .iter()
        .map(|outcome| match outcome {
            SideOutcome::Left => 1.0,
            SideOutcome::Right => 0.0,
            SideOutcome::Tie => 0.5,
        })
        .sum::<f64>();
    let rate = left_points / outcomes.len() as f64;
    let enough = outcomes.len() >= 6;
    let detected = enough && (rate - 0.5).abs() >= 0.20;
    PositionBiasSummary {
        sample_size: outcomes.len(),
        left_preference_rate: Some(rate),
        detected,
        warning: if enough {
            detected
                .then(|| "Left/right preference is large enough to threaten the comparison".into())
        } else {
            Some("Fewer than 6 votes; position bias is unknown".into())
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisagreementSummary {
    pub paired_sample_size: usize,
    pub disagreements: usize,
    pub disagreement_rate: Option<f64>,
    pub sufficient_sample: bool,
    pub warning: Option<String>,
}

pub fn judge_disagreement(pairs: &[(SideOutcome, SideOutcome)]) -> DisagreementSummary {
    let disagreements = pairs.iter().filter(|(human, model)| human != model).count();
    let paired_sample_size = pairs.len();
    let sufficient_sample = paired_sample_size >= MIN_CONFIDENCE_SAMPLES;
    DisagreementSummary {
        paired_sample_size,
        disagreements,
        disagreement_rate: (paired_sample_size > 0)
            .then_some(disagreements as f64 / paired_sample_size as f64),
        sufficient_sample,
        warning: (!sufficient_sample).then(|| {
            "Too few paired human and auto-judge decisions to interpret disagreement".into()
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EloEligibility {
    pub run_complete: bool,
    pub all_trials_valid: bool,
    pub comparable: bool,
    pub positions_balanced: bool,
    pub human_judged: bool,
    pub sample_size: usize,
}

pub fn is_elo_eligible(input: EloEligibility) -> bool {
    input.run_complete
        && input.all_trials_valid
        && input.comparable
        && input.positions_balanced
        && input.human_judged
        && input.sample_size >= 6
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(repetitions: u32) -> EvaluationConfig {
        EvaluationConfig {
            repetitions,
            warmup_repetitions: 1,
            timeout_seconds: 30,
            temperature: 0.2,
            num_predict: Some(32),
            think: false,
            seed: Some(42),
        }
    }

    #[test]
    fn randomized_plan_is_deterministic_for_recorded_seed() {
        let first = build_trial_plan(&[10, 20], &[1, 2], &config(3), 42).unwrap();
        let replay = build_trial_plan(&[10, 20], &[1, 2], &config(3), 42).unwrap();
        let different = build_trial_plan(&[10, 20], &[1, 2], &config(3), 43).unwrap();
        assert_eq!(first, replay);
        assert_ne!(first, different);
    }

    #[test]
    fn positions_are_balanced() {
        for count in 1..20 {
            let positions = balanced_positions(count, 7);
            let left = positions.iter().filter(|position| **position).count();
            let right = count - left;
            assert!((left as isize - right as isize).abs() <= 1);
        }
    }

    #[test]
    fn warmups_are_excluded_from_measured_count() {
        let plan = build_trial_plan(&[10, 20], &[1, 2], &config(3), 42).unwrap();
        assert_eq!(
            plan.iter()
                .filter(|item| item.kind == TrialKind::Warmup)
                .count(),
            2
        );
        assert_eq!(
            plan.iter()
                .filter(|item| item.kind == TrialKind::Measured)
                .count(),
            12
        );
    }

    #[test]
    fn maximum_supported_trial_plan_is_balanced_and_deterministic() {
        let prompts: Vec<i64> = (1..=15).collect();
        let config = EvaluationConfig {
            repetitions: 20,
            warmup_repetitions: 3,
            timeout_seconds: 3_600,
            temperature: 2.0,
            num_predict: Some(32_768),
            think: true,
            seed: Some(9_001),
        };
        let first = build_trial_plan(&prompts, &[101, 202], &config, 9_001).unwrap();
        let replay = build_trial_plan(&prompts, &[101, 202], &config, 9_001).unwrap();

        assert_eq!(first, replay);
        assert_eq!(first.len(), 606);
        assert_eq!(
            first
                .iter()
                .filter(|trial| trial.kind == TrialKind::Measured)
                .count(),
            600
        );
        assert_eq!(
            first
                .iter()
                .filter(|trial| trial.comparison_position.as_deref() == Some("left"))
                .count(),
            300
        );
        assert_eq!(
            first
                .iter()
                .filter(|trial| trial.comparison_position.as_deref() == Some("right"))
                .count(),
            300
        );
        assert!(first
            .iter()
            .enumerate()
            .all(|(index, trial)| trial.execution_order == index));
    }

    #[test]
    fn confidence_requires_repeated_samples() {
        let one = mean_confidence_95(&[8.0]);
        assert!(!one.sufficient_sample);
        assert!(one.lower_95.is_none());
        let repeated = mean_confidence_95(&[7.0, 8.0, 9.0]);
        assert!(repeated.sufficient_sample);
        assert_eq!(repeated.mean, Some(8.0));
        assert!(repeated.lower_95.unwrap() < 8.0);
        assert!(repeated.upper_95.unwrap() > 8.0);
    }

    #[test]
    fn score_confidence_stays_inside_the_score_domain() {
        let upper = mean_confidence_95(&[10.0, 10.0, 2.0, 10.0, 10.0]);
        assert_eq!(upper.upper_95, Some(10.0));

        let lower = mean_confidence_95(&[1.0, 1.0, 9.0, 1.0, 1.0]);
        assert_eq!(lower.lower_95, Some(1.0));
    }

    #[test]
    fn invalid_or_incomplete_trials_are_never_elo_eligible() {
        let eligible = EloEligibility {
            run_complete: true,
            all_trials_valid: true,
            comparable: true,
            positions_balanced: true,
            human_judged: true,
            sample_size: 6,
        };
        assert!(is_elo_eligible(eligible));
        assert!(!is_elo_eligible(EloEligibility {
            all_trials_valid: false,
            ..eligible
        }));
        assert!(!is_elo_eligible(EloEligibility {
            run_complete: false,
            ..eligible
        }));
        assert!(!is_elo_eligible(EloEligibility {
            comparable: false,
            ..eligible
        }));
    }

    #[test]
    fn judge_disagreement_is_explicit() {
        let summary = judge_disagreement(&[
            (SideOutcome::Left, SideOutcome::Right),
            (SideOutcome::Right, SideOutcome::Right),
            (SideOutcome::Tie, SideOutcome::Left),
        ]);
        assert_eq!(summary.disagreements, 2);
        assert_eq!(summary.disagreement_rate, Some(2.0 / 3.0));
        assert!(summary.sufficient_sample);
    }

    #[test]
    fn position_bias_stays_unknown_for_small_samples() {
        let summary = detect_position_bias(&[SideOutcome::Left; 5]);
        assert!(!summary.detected);
        assert!(summary.warning.unwrap().contains("unknown"));
        let detected = detect_position_bias(&[SideOutcome::Left; 6]);
        assert!(detected.detected);
    }

    #[test]
    fn digest_is_stable_for_fixture_replay() {
        let fixture = serde_json::json!({"prompt":"2+2", "output":"4", "seed":42});
        assert_eq!(
            digest_json(&fixture).unwrap(),
            digest_json(&fixture).unwrap()
        );
    }
}
