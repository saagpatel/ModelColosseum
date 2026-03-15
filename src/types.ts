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
