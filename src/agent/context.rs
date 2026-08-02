use crate::service::mlx::Message;

pub struct ContextManager {
    max_turns: u32,
    messages: Vec<Message>,
}

impl ContextManager {
    pub fn new(max_turns: u32) -> Self {
        Self {
            max_turns,
            messages: Vec::new(),
        }
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
    }

    pub fn add_system_message(&mut self, content: &str) {
        self.messages.push(Message {
            role: "system".to_string(),
            content: content.to_string(),
        });
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
        let max_messages = (self.max_turns * 2) as usize;
        if self.messages.len() > max_messages {
            let drain_count = self.messages.len() - max_messages;
            self.messages.drain(..drain_count);
        }
    }

    #[allow(dead_code)]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
