/// Outcome from the perspective of player A.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Win,  // player A wins
    Loss, // player A loses
    Draw,
}

impl Outcome {
    fn score(self) -> f64 {
        match self {
            Outcome::Win => 1.0,
            Outcome::Loss => 0.0,
            Outcome::Draw => 0.5,
        }
    }
}

/// Expected score for player A given ratings of A and B.
pub fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf((rating_b - rating_a) / 400.0))
}

/// Select K-factor based on total games played.
/// K=40 for < 10 games, K=32 for 10-30, K=24 for > 30.
pub fn k_factor_for_games(total_games: u32) -> f64 {
    if total_games < 10 {
        40.0
    } else if total_games <= 30 {
        32.0
    } else {
        24.0
    }
}

/// Update ratings for both players after a match.
/// Returns (new_rating_a, new_rating_b, k_a, k_b).
pub fn update_ratings(
    rating_a: f64,
    rating_b: f64,
    outcome: Outcome,
    games_a: u32,
    games_b: u32,
) -> (f64, f64, f64, f64) {
    let k_a = k_factor_for_games(games_a);
    let k_b = k_factor_for_games(games_b);

    let expected_a = expected_score(rating_a, rating_b);
    let expected_b = 1.0 - expected_a;

    let score_a = outcome.score();
    let score_b = 1.0 - score_a;

    let new_a = rating_a + k_a * (score_a - expected_a);
    let new_b = rating_b + k_b * (score_b - expected_b);

    (new_a, new_b, k_a, k_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_ratings_draw_no_change() {
        let (new_a, new_b, _, _) = update_ratings(1500.0, 1500.0, Outcome::Draw, 50, 50);
        assert!(
            (new_a - 1500.0).abs() < 0.001,
            "A should stay ~1500, got {new_a}"
        );
        assert!(
            (new_b - 1500.0).abs() < 0.001,
            "B should stay ~1500, got {new_b}"
        );
    }

    #[test]
    fn equal_ratings_win_updates_symmetrically() {
        let (new_a, new_b, _, _) = update_ratings(1500.0, 1500.0, Outcome::Win, 50, 50);
        // With K=24, equal ratings: change = 24 * (1.0 - 0.5) = 12
        assert!(
            (new_a - 1512.0).abs() < 0.001,
            "A should be ~1512, got {new_a}"
        );
        assert!(
            (new_b - 1488.0).abs() < 0.001,
            "B should be ~1488, got {new_b}"
        );
    }

    #[test]
    fn strong_beats_weak_small_change() {
        // Strong (1800) beats weak (1200) — expected, so small Elo gain
        let (new_a, new_b, _, _) = update_ratings(1800.0, 1200.0, Outcome::Win, 50, 50);
        let change_a = new_a - 1800.0;
        assert!(
            change_a > 0.0 && change_a < 3.0,
            "Strong winner gains little, got +{change_a}"
        );
        let change_b = 1200.0 - new_b;
        assert!(
            change_b > 0.0 && change_b < 3.0,
            "Weak loser loses little, got -{change_b}"
        );
    }

    #[test]
    fn weak_beats_strong_large_change() {
        // Weak (1200) beats strong (1800) — upset, big swing
        let (new_a, _new_b, _, _) = update_ratings(1200.0, 1800.0, Outcome::Win, 50, 50);
        let change_a = new_a - 1200.0;
        assert!(change_a > 21.0, "Upset winner gains big, got +{change_a}");
    }

    #[test]
    fn k_factor_transitions() {
        assert!((k_factor_for_games(0) - 40.0).abs() < 0.001);
        assert!((k_factor_for_games(5) - 40.0).abs() < 0.001);
        assert!((k_factor_for_games(9) - 40.0).abs() < 0.001);
        assert!((k_factor_for_games(10) - 32.0).abs() < 0.001);
        assert!((k_factor_for_games(20) - 32.0).abs() < 0.001);
        assert!((k_factor_for_games(30) - 32.0).abs() < 0.001);
        assert!((k_factor_for_games(31) - 24.0).abs() < 0.001);
        assert!((k_factor_for_games(100) - 24.0).abs() < 0.001);
    }

    #[test]
    fn k_factor_affects_magnitude() {
        // New player (K=40) vs veteran (K=24) — same outcome, different magnitude
        let (new_a, _, k_a, k_b) = update_ratings(1500.0, 1500.0, Outcome::Win, 5, 50);
        assert!((k_a - 40.0).abs() < 0.001);
        assert!((k_b - 24.0).abs() < 0.001);
        let change_a = new_a - 1500.0;
        assert!(
            (change_a - 20.0).abs() < 0.001,
            "K=40 player gains 20, got {change_a}"
        );
    }

    #[test]
    fn expected_score_equal_ratings() {
        let e = expected_score(1500.0, 1500.0);
        assert!(
            (e - 0.5).abs() < 0.001,
            "Equal ratings → 0.5 expected, got {e}"
        );
    }

    #[test]
    fn expected_score_400_diff() {
        // 400 point difference → ~0.909 expected for the stronger player
        let e = expected_score(1900.0, 1500.0);
        assert!((e - 0.909).abs() < 0.01, "400pt advantage → ~0.91, got {e}");
    }

    #[test]
    fn equal_ratings_loss_updates_symmetrically() {
        let (new_a, new_b, _, _) = update_ratings(1500.0, 1500.0, Outcome::Loss, 50, 50);
        // With K=24, equal ratings: A loses 12, B gains 12
        assert!(
            (new_a - 1488.0).abs() < 0.001,
            "A should be ~1488 after loss, got {new_a}"
        );
        assert!(
            (new_b - 1512.0).abs() < 0.001,
            "B should be ~1512 after A's loss, got {new_b}"
        );
    }

    #[test]
    fn loss_is_inverse_of_win() {
        let (win_a, win_b, _, _) = update_ratings(1500.0, 1500.0, Outcome::Win, 50, 50);
        let (loss_a, loss_b, _, _) = update_ratings(1500.0, 1500.0, Outcome::Loss, 50, 50);
        // Win for A should mirror loss for A
        assert!(
            (win_a - loss_b).abs() < 0.001,
            "A's win rating should equal B's loss-counterpart rating"
        );
        assert!(
            (win_b - loss_a).abs() < 0.001,
            "B's win-counterpart rating should equal A's loss rating"
        );
    }

    #[test]
    fn ratings_conserved_on_draw() {
        // Total Elo in system should be preserved on draw with same K-factor
        let (new_a, new_b, _, _) = update_ratings(1600.0, 1400.0, Outcome::Draw, 50, 50);
        let total_before = 1600.0 + 1400.0;
        let total_after = new_a + new_b;
        assert!(
            (total_before - total_after).abs() < 0.001,
            "Elo should be conserved: {total_before} vs {total_after}"
        );
    }
}
