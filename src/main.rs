//! Rusty — a small chatbot.
//!
//! * With `GROQ_API_KEY` set (or `RUSTY_API_KEY` + `RUSTY_API_URL` for any
//!   OpenAI-compatible endpoint) it answers any question via an LLM and keeps
//!   conversation history.
//! * Without a key it falls back to a built-in rule-based mode for basic chat.
//!
//! Usage:
//!   rusty                 # interactive chat
//!   rusty "your question" # one-shot answer (used by GitHub Actions)

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Write};

const DEFAULT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const DEFAULT_MODEL: &str = "openai/gpt-oss-120b";
const SYSTEM_PROMPT: &str = "You are Rusty, a friendly, concise assistant. \
Answer questions accurately and clearly. If you are unsure, say so.";
const MAX_HISTORY: usize = 20;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,
    content: String,
}

impl Message {
    fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

struct LlmClient {
    http: reqwest::blocking::Client,
    url: String,
    key: String,
    model: String,
}

impl LlmClient {
    fn from_env() -> Option<Self> {
        let key = env::var("RUSTY_API_KEY")
            .or_else(|_| env::var("GROQ_API_KEY"))
            .ok()
            .filter(|k| !k.trim().is_empty())?;
        Some(Self {
            http: reqwest::blocking::Client::new(),
            url: env::var("RUSTY_API_URL").unwrap_or_else(|_| DEFAULT_URL.into()),
            key,
            model: env::var("RUSTY_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into()),
        })
    }

    fn chat(&self, messages: &[Message]) -> Result<String, String> {
        let body = ChatRequest {
            model: &self.model,
            messages,
            temperature: 0.7,
        };
        let resp = self
            .http
            .post(&self.url)
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .map_err(|e| format!("network error: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(format!("API error {status}: {text}"));
        }
        let parsed: ChatResponse = resp.json().map_err(|e| format!("bad response: {e}"))?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| "empty response".into())
    }
}

/// Offline fallback for basic conversation.
fn rule_based_reply(input: &str) -> String {
    let lower = input.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));

    if has(&["hello", "hi ", "hey"]) || lower == "hi" {
        "Hey there! I'm Rusty. What's on your mind?".into()
    } else if has(&["how are you", "how r u"]) {
        "Running smoothly, zero segfaults. How about you?".into()
    } else if has(&["your name", "who are you"]) {
        "I'm Rusty, a chatbot written in Rust.".into()
    } else if has(&["thank"]) {
        "Anytime!".into()
    } else if has(&["joke"]) {
        "Why did the Rust programmer never get lost? The borrow checker always knew where things were.".into()
    } else if has(&["time", "date"]) {
        "I can't read the clock in offline mode — try `date` in your terminal.".into()
    } else if lower.ends_with('?') {
        "Good question! Set GROQ_API_KEY so I can answer properly — offline I only do small talk."
            .into()
    } else {
        "Interesting — tell me more.".into()
    }
}

struct Rusty {
    llm: Option<LlmClient>,
    history: Vec<Message>,
}

impl Rusty {
    fn new() -> Self {
        Self {
            llm: LlmClient::from_env(),
            history: vec![Message::new("system", SYSTEM_PROMPT)],
        }
    }

    fn reply(&mut self, input: &str) -> String {
        let Some(llm) = &self.llm else {
            return rule_based_reply(input);
        };
        self.history.push(Message::new("user", input));
        // keep system prompt + last MAX_HISTORY messages
        if self.history.len() > MAX_HISTORY + 1 {
            let excess = self.history.len() - (MAX_HISTORY + 1);
            self.history.drain(1..=excess);
        }
        match llm.chat(&self.history) {
            Ok(answer) => {
                self.history.push(Message::new("assistant", &answer));
                answer
            }
            Err(e) => {
                self.history.pop();
                format!("[{e}] {}", rule_based_reply(input))
            }
        }
    }
}

fn main() {
    let mut bot = Rusty::new();
    let args: Vec<String> = env::args().skip(1).collect();

    // One-shot mode
    if !args.is_empty() {
        println!("{}", bot.reply(&args.join(" ")));
        return;
    }

    let mode = if bot.llm.is_some() { "LLM" } else { "offline" };
    println!("🦀 Rusty ({mode} mode). Type 'exit' to quit, 'reset' to clear memory.");
    let stdin = io::stdin();
    loop {
        print!("you> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let input = line.trim();
        match input {
            "" => continue,
            "exit" | "quit" => break,
            "reset" => {
                bot.history.truncate(1);
                println!("rusty> Memory cleared.");
            }
            _ => println!("rusty> {}", bot.reply(input)),
        }
    }
    println!("Bye! 👋");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets() {
        assert!(rule_based_reply("hello").contains("Rusty"));
    }

    #[test]
    fn questions_without_key_hint_setup() {
        assert!(rule_based_reply("what is gravity?").contains("GROQ_API_KEY"));
    }

    #[test]
    fn offline_bot_replies() {
        let mut bot = Rusty {
            llm: None,
            history: vec![],
        };
        assert!(!bot.reply("hi").is_empty());
    }
}
