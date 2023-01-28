pub mod config;

use std::io::{self, Write};
use std::time::UNIX_EPOCH;
use std::{fmt::Display, fs::File};

use config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Choice {
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct AIResponse {
    choices: Vec<Choice>,
}

impl Display for AIResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.choices
                .get(0)
                .expect("should have gotten at least one choice from Openai")
                .text
                .trim()
        )
    }
}

pub trait Chat {
    fn say(&mut self, text: String) -> AIResponse;
}

#[derive(Debug)]
pub struct ChatWithAI {
    c: Vec<String>,
    secrets: Config,
}

impl Chat for ChatWithAI {
    fn say(&mut self, text: String) -> AIResponse {
        self.c.push(text.to_string());

        let req = ureq::json!({
          "model": "text-davinci-003",
          "prompt": self.as_prompt(),
          "temperature": 0.9,
          "max_tokens": 150,
          "top_p": 1,
          "frequency_penalty": 0.0,
          "presence_penalty": 0.6,
          "stop": ["You:", "Sky:"]
        });

        let res: AIResponse = ureq::post("https://api.openai.com/v1/completions")
            .set(
                "Authorization",
                &format!(
                    "Bearer {}",
                    self.secrets.api_key.clone().expect(
                        "api key should be confirmed to exist before calling this function"
                    )
                ),
            )
            .send_json(req)
            .expect("ureq works")
            .into_json()
            .expect("ureq works");

        self.c.push(res.to_string());

        res
    }
}

impl ChatWithAI {
    pub fn new(secrets: Config) -> Self {
        Self { c: vec![], secrets }
    }

    fn dialogue(&self) -> String {
        self.c
            .chunks(2)
            .filter_map(|chunk| match chunk {
                [h, a] => Some(format!("You: {h}\nSky: {a}\n")),
                [h] => Some(format!("You: {h}\nSky:")),
                _ => None,
            })
            .collect()
    }

    fn as_prompt(&self) -> String {
        const PRELUDE: &str = "The following is a conversation between you and an AI assistant named Sky. Sky is helpful, creative, clever, and very friendly.";
        let dialogue = self.dialogue();

        format!("{PRELUDE}\n{dialogue}")
    }
}

impl Display for ChatWithAI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.dialogue())
    }
}

pub struct ReportingToFile {
    chat: ChatWithAI,
    file: File,
}

impl ReportingToFile {
    pub fn new(chat: ChatWithAI, file: File) -> Self {
        Self { chat, file }
    }
}

impl Chat for ReportingToFile {
    fn say(&mut self, text: String) -> AIResponse {
        self.file
            .write_fmt(format_args!("\nYou: {}\n", text))
            .map_err(|e| eprintln!("{e}"))
            .ok();
        let res = self.chat.say(text);
        self.file
            .write_fmt(format_args!("\nSky: {}\n", res.to_string()))
            .map_err(|e| eprintln!("{e}"))
            .ok();

        self.file.flush().map_err(|e| eprintln!("{e}")).ok();
        res
    }
}

#[inline]
pub fn chat_factory(cfg: Config, report: bool) -> io::Result<Box<dyn Chat>> {
    match cfg.api_key {
        Some(_) => {
            if report {
                let now = UNIX_EPOCH
                    .elapsed()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                    .as_millis();
                let file = File::create(format!("./chat-with-sky-{now}"))?;
                return Ok(Box::new(ReportingToFile::new(ChatWithAI::new(cfg), file)));
            } else {
                return Ok(Box::new(ChatWithAI::new(cfg)));
            }
        }
        None => panic!("need an api key"),
    }
}
