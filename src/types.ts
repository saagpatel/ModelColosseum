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
