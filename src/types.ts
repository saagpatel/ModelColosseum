export interface Model {
  id: number;
  name: string;
  display_name: string;
  parameter_count: number | null;
  quantization: string | null;
  family: string | null;
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
