use crate::service::mlx::Message;

// A5 修复: Agent 上下文之前仅按消息条数截断且只在 add_user_message 触发;
// add_assistant_message / add_system_message (工具结果可达数十 KB) 不参与截断,
// 10 轮工具密集调用后撞破 max_position_embeddings → 推理 400/413 失败无降级。
// 改为: 每次添加都 trim; 除条数上限外加 token 估算预算, 超预算从最旧消息起丢弃。
// 粗估 1 token ≈ 4 字符 (英文), 中文偏保守但作为降级阈值足够。

const CHARS_PER_TOKEN: usize = 4;
const DEFAULT_MAX_TOKENS: u32 = 8192;
// 预留生成空间: 上下文预算 = max_tokens 的 75%, 余 25% 给输出。
const CONTEXT_BUDGET_RATIO: f64 = 0.75;

pub struct ContextManager {
    max_turns: u32,
    max_tokens: u32,
    messages: Vec<Message>,
}

impl ContextManager {
    pub fn new(max_turns: u32) -> Self {
        Self {
            max_turns,
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: Vec::new(),
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        if max_tokens > 0 {
            self.max_tokens = max_tokens;
        }
        self
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
        });
        self.trim();
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: content.to_string(),
        });
        // A5: 工具/助手消息也必须 trim, 否则无界增长。
        self.trim();
    }

    pub fn add_system_message(&mut self, content: &str) {
        self.messages.push(Message {
            role: "system".to_string(),
            content: content.to_string(),
        });
        // A5: system 消息 (工具结果) 同样 trim。
        self.trim();
    }

    pub fn build_messages(&self, system_prompt: &Option<String>) -> Vec<Message> {
        let mut result = Vec::new();
        if let Some(prompt) = system_prompt {
            result.push(Message {
                role: "system".to_string(),
                content: prompt.clone(),
            });
        }
        result.extend(self.messages.clone());
        result
    }

    fn trim(&mut self) {
        // 第一道: 按消息条数上限 (max_turns*2), 防轮次爆炸。
        let max_messages = (self.max_turns * 2) as usize;
        if self.messages.len() > max_messages {
            let drain_count = self.messages.len() - max_messages;
            self.messages.drain(..drain_count);
        }
        // 第二道: 按 token 预算。从最旧消息起丢弃, 直至总估算 token 落入预算。
        let budget = ((self.max_tokens as f64) * CONTEXT_BUDGET_RATIO) as usize;
        while self.estimated_tokens() > budget && self.messages.len() > 1 {
            self.messages.remove(0);
        }
    }

    fn estimated_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| (m.content.len() / CHARS_PER_TOKEN).max(1))
            .sum()
    }

    #[allow(dead_code)]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_by_message_count() {
        let mut ctx = ContextManager::new(2);
        for i in 0..10 {
            ctx.add_user_message(&format!("msg {}", i));
        }
        assert!(ctx.message_count() <= 4, "should cap at max_turns*2");
    }

    #[test]
    fn test_trim_assistant_and_system_messages() {
        // A5 回归: 之前 assistant/system 不 trim, 现在必须。
        let mut ctx = ContextManager::new(10).with_max_tokens(100);
        for _ in 0..50 {
            ctx.add_assistant_message(&"x".repeat(200));
        }
        // 预算 75 token → 远小于 50*50 token, 必须被截断到很少消息。
        assert!(
            ctx.message_count() < 10,
            "assistant messages must be token-trimmed"
        );
    }

    #[test]
    fn test_token_budget_keeps_recent() {
        let mut ctx = ContextManager::new(100).with_max_tokens(100);
        ctx.add_user_message("old old old old old old old old old old");
        ctx.add_assistant_message("new new new new new new new new new new");
        // 预算 ~75 token, 两条共约 20 token, 都应保留。
        assert_eq!(ctx.message_count(), 2);
    }

    #[test]
    fn test_empty_context() {
        let ctx = ContextManager::new(10);
        assert_eq!(ctx.message_count(), 0);
    }
}
