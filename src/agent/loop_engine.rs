pub struct LoopStats {
    pub turns_used: u32,
    pub tools_called: u32,
    pub tokens_used: u32,
}

impl LoopStats {
    pub fn new() -> Self {
        Self {
            turns_used: 0,
            tools_called: 0,
            tokens_used: 0,
        }
    }

    pub fn record_turn(&mut self, tokens: u32) {
        self.turns_used += 1;
        self.tokens_used += tokens;
    }

    pub fn record_tool_call(&mut self) {
        self.tools_called += 1;
    }

    pub fn summary(&self) -> String {
        format!(
            "Turns: {}, Tools: {}, Tokens: {}",
            self.turns_used, self.tools_called, self.tokens_used
        )
    }
}
