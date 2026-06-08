pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    chars.div_ceil(4)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenTracker {
    input_total: usize,
    output_total: usize,
    turn_input: usize,
    turn_output: usize,
    input_cost_per_1m: f64,
    output_cost_per_1m: f64,
}

impl TokenTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_costs(input_cost: f64, output_cost: f64) -> Self {
        Self {
            input_cost_per_1m: input_cost,
            output_cost_per_1m: output_cost,
            ..Default::default()
        }
    }

    pub fn add_input(&mut self, tokens: usize) {
        self.input_total += tokens;
        self.turn_input += tokens;
    }

    pub fn add_output(&mut self, tokens: usize) {
        self.output_total += tokens;
        self.turn_output += tokens;
    }

    pub fn end_turn(&mut self) {
        self.turn_input = 0;
        self.turn_output = 0;
    }

    pub fn session_total(&self) -> usize {
        self.input_total + self.output_total
    }

    pub fn turn_total(&self) -> usize {
        self.turn_input + self.turn_output
    }

    pub fn input_total(&self) -> usize {
        self.input_total
    }

    pub fn output_total(&self) -> usize {
        self.output_total
    }

    pub fn session_cost(&self) -> f64 {
        let input_cost = self.input_total as f64 * self.input_cost_per_1m / 1_000_000.0;
        let output_cost = self.output_total as f64 * self.output_cost_per_1m / 1_000_000.0;
        input_cost + output_cost
    }
}
