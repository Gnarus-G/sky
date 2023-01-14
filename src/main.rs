use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    io::{stdin, stdout, Write},
    vec,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    api_key: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { api_key: None }
    }
}

/// An AI chat assistant powered by Openai.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Set some runtime configuration, most importantly the Openai API KEY
    Config {
        /// Openai API KEY
        #[arg(short = 'a', long)]
        api_key: Option<String>,

        /// Print all the current configurations.
        #[arg(long)]
        show: bool,
    },
}

#[derive(Debug, Deserialize)]
struct Choice {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AIResponse {
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

#[derive(Debug)]
struct ChatWithAI {
    c: Vec<String>,
    secrets: Config,
}

impl ChatWithAI {
    fn new(secrets: Config) -> Self {
        Self { c: vec![], secrets }
    }

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

fn main() {
    let mut chat = match Cli::parse().command {
        Some(Command::Config { api_key, show }) => {
            if api_key.is_some() {
                confy::store("sky", None, Config { api_key }).unwrap();
            }

            if show {
                println!("{:?}", confy::load::<Config>("sky", None).unwrap())
            }

            return;
        }
        None => {
            let cfg: Config = confy::load("sky", None).unwrap();
            match cfg.api_key {
                Some(_) => ChatWithAI::new(cfg),
                None => panic!("need an api key"),
            }
        }
    };

    prompt();
    for line in stdin().lines().flatten() {
        let response = chat.say(line);
        println!("\n{response}\n");
        prompt();
    }
}

#[inline(always)]
fn prompt() {
    print!("> ");
    stdout().flush().ok();
}
