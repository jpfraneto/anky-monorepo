/// Estimate USD cost for Claude API calls.
/// Based on claude-sonnet-4-20250514 pricing: $3/MTok input, $15/MTok output.
pub fn estimate_claude_cost(input_tokens: i64, output_tokens: i64) -> f64 {
    (input_tokens as f64 * 3.0 / 1_000_000.0) + (output_tokens as f64 * 15.0 / 1_000_000.0)
}

/// Estimate cost for a single Anky generation (prompt + reflection + title + image).
pub fn estimate_single_anky_cost() -> f64 {
    // Claude: ~2000 input + ~500 output (prompt) + ~2000 input + ~2000 output (reflection) + ~3000 input + ~50 output (title)
    let claude_cost = estimate_claude_cost(7000, 2550);
    // Gemini image gen: ~$0.04 per image (approximate)
    let gemini_cost = 0.04;
    claude_cost + gemini_cost
}

/// Calculate the cost for a writing transformation with 50% markup.
pub fn calculate_transform_cost(input_tokens: i64, output_tokens: i64) -> f64 {
    let base_cost = estimate_claude_cost(input_tokens, output_tokens);
    base_cost * 1.5 // 50% markup
}

/// Estimate cost for a full 88-being collection.
pub fn estimate_collection_cost(num_beings: usize) -> f64 {
    // Each being: stream generation + prompt + reflection + title + image
    let per_being = estimate_single_anky_cost()
        + estimate_claude_cost(500, 2000); // stream generation
    let generation_cost = per_being * num_beings as f64;
    // Training cost estimate
    let training_cost = 2.0; // Electricity cost estimate for 4000 steps on 2x4090
    generation_cost + training_cost
}
