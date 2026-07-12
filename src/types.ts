export type DebateFormat = "freestyle" | "formal" | "socratic";

export interface Setting {
  key: string;
  value: string;
}

export interface Model {
  id: number;
  name: string;
  display_name: string;
  parameter_count: number | null;
  quantization: string | null;
  family: string | null;
  digest: string | null;
  size_bytes: number | null;
  elo_rating: number;
  arena_wins: number;
  arena_losses: number;
  arena_draws: number;
  total_debates: number;
  last_used_at: string | null;
}

export interface StreamTokenPayload {
  debate_id: number;
  round: number;
  token: string;
}

export interface RoundCompletePayload {
  debate_id: number;
  round: number;
  model_a_content: string;
  model_b_content: string;
}

export interface DebateCompletePayload {
  debate_id: number;
  total_rounds: number;
}

export interface DebateErrorPayload {
  debate_id: number;
  message: string;
}

export interface DebateAbortedPayload {
  debate_id: number;
}

export interface DebateModePayload {
  debate_id: number;
  mode: "concurrent" | "sequential";
}

export interface VoteResult {
  debate_id: number;
  rating_a_before: number;
  rating_a_after: number;
  rating_b_before: number;
  rating_b_after: number;
}

export interface EloHistoryPoint {
  rating: number;
  created_at: string;
}

export interface DebateSummary {
  id: number;
  topic: string;
  model_a_name: string;
  model_b_name: string;
  winner: string | null;
  status: string;
  total_rounds: number;
  created_at: string;
}

export interface RoundTranscript {
  round_number: number;
  speaker: string;
  phase: string;
  content: string;
}

export interface TestSuite {
  id: number;
  name: string;
  description: string | null;
  is_default: number;
  created_at: string;
  updated_at: string;
}

export interface Prompt {
  id: number;
  suite_id: number;
  category: string;
  title: string;
  text: string;
  system_prompt: string | null;
  ideal_answer: string | null;
  eval_criteria: string | null;
  sort_order: number;
  created_at: string;
}

export interface BenchmarkProgress {
  run_id: number;
  completed: number;
  total: number;
  current_model: string;
  current_prompt: string;
}

export interface BenchmarkStreamPayload {
  run_id: number;
  token: string;
}

export interface BenchmarkCompletePayload {
  run_id: number;
}

export interface BenchmarkErrorPayload {
  run_id: number;
  message: string;
}

export interface BenchmarkResult {
  id: number;
  run_id: number;
  prompt_id: number;
  model_id: number;
  model_name: string;
  prompt_title: string;
  prompt_category: string;
  output: string;
  tokens_generated: number;
  time_to_first_token_ms: number | null;
  total_time_ms: number;
  tokens_per_second: number | null;
  manual_score: number | null;
  auto_judge_score: number | null;
  auto_judge_notes: string | null;
  repetition_index: number;
  trial_key: string | null;
  generation_seed: number | null;
  trial_status: string;
  created_at: string;
}

export interface EvaluationConfig {
  repetitions: number;
  warmup_repetitions: number;
  timeout_seconds: number;
  temperature: number;
  num_predict: number | null;
  think: boolean;
  seed: number | null;
}

export interface BenchmarkRunSummary {
  id: number;
  suite_id: number;
  suite_name: string;
  status: string;
  model_count: number;
  prompt_count: number;
  scored_count: number;
  total_results: number;
  started_at: string;
  completed_at: string | null;
  outcome_status: string;
  repetitions: number;
  failed_count: number;
  excluded_count: number;
  comparable: boolean;
  comparability_notes: string | null;
  manifest_digest: string | null;
}

export interface ConfidenceSummary {
  sample_size: number;
  mean: number | null;
  lower_95: number | null;
  upper_95: number | null;
  sufficient_sample: boolean;
  warning: string | null;
}

export interface CapabilityEvidence {
  category: string;
  model_id: number;
  model_name: string;
  scoring_method: string;
  confidence: ConfidenceSummary;
}

export interface CapabilityRecommendation {
  category: string;
  recommended_model: string | null;
  confidence: string;
  reason: string;
}

export interface RunEvidence {
  run_id: number;
  outcome_status: string;
  manifest_digest: string | null;
  comparable: boolean;
  comparability_notes: string | null;
  planned_measured_trials: number;
  completed_measured_trials: number;
  failed_trials: number;
  excluded_trials: number;
  cancelled_trials: number;
  timeout_trials: number;
  hardware_dependent: boolean;
  capability_evidence: CapabilityEvidence[];
  recommendations: CapabilityRecommendation[];
  position_bias: {
    sample_size: number;
    left_preference_rate: number | null;
    detected: boolean;
    warning: string | null;
  };
  judge_disagreement: {
    paired_sample_size: number;
    disagreements: number;
    disagreement_rate: number | null;
    sufficient_sample: boolean;
    warning: string | null;
  };
  judge_provenance: string[];
  elo_eligible: boolean;
  elo_updated: boolean;
}

export interface RunComparability {
  comparable: boolean;
  classification: "exact_reproduction" | "hardware_variant" | "runtime_variant" | "incomparable";
  quality_comparable: boolean;
  performance_comparable: boolean;
  reasons: string[];
}

export interface BenchmarkLeaderboardEntry {
  model_id: number;
  model_name: string;
  display_name: string;
  avg_score: number | null;
  category_scores: Record<string, number>;
  avg_tps: number | null;
  avg_ttft_ms: number | null;
  total_prompts_scored: number;
}

export interface RunComparisonEntry {
  prompt_id: number;
  prompt_title: string;
  prompt_category: string;
  model_id: number;
  model_name: string;
  run_a_score: number | null;
  run_b_score: number | null;
  score_delta: number | null;
}

export interface AutoJudgeProgressPayload {
  run_id: number;
  completed: number;
  total: number;
  current_model: string;
}

export interface AutoJudgeCompletePayload {
  run_id: number;
  scores_added: number;
}

export interface SparringStartedPayload {
  debate_id: number;
  first_phase: string;
  word_limit: number;
}

export interface SparringRoundCompletePayload {
  debate_id: number;
  round: number;
  phase: string;
  ai_content: string;
  next_phase: string | null;
  next_word_limit: number | null;
  is_complete: boolean;
}

export interface SparringErrorPayload {
  debate_id: number;
  message: string;
}

export interface SparringScorecard {
  debate_id: number;
  human_persuasiveness: number;
  human_evidence: number;
  human_coherence: number;
  human_rebuttal: number;
  ai_persuasiveness: number;
  ai_evidence: number;
  ai_coherence: number;
  ai_rebuttal: number;
  strongest_human_point: string;
  weakest_human_point: string;
  missed_argument: string;
  improvement_tip: string;
  raw_judge_output: string;
}

export interface UserStats {
  elo_rating: number;
  total_debates: number;
  wins: number;
  losses: number;
  draws: number;
}

export interface BlindPair {
  comparison_id: number;
  prompt_id: number;
  repetition_index: number;
  prompt_title: string;
  prompt_category: string;
  left_result_id: number;
  left_output: string;
  right_result_id: number;
  right_output: string;
}

export interface BlindComparison {
  id: number;
  pairs: BlindPair[];
}

export interface BlindRevealEntry {
  prompt_id: number;
  prompt_title: string;
  model_a_name: string;
  model_b_name: string;
  winner: string;
}

export interface BlindReveal {
  model_a_name: string;
  model_b_name: string;
  model_a_wins: number;
  model_b_wins: number;
  ties: number;
  entries: BlindRevealEntry[];
}

export interface BenchmarkMetricsPayload {
  run_id: number;
  cpu_percent: number;
  memory_percent: number;
  swap_percent: number;
  timestamp_ms: number;
}
