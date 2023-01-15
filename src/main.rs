use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    io::{self, stdin, stdout, Write},
    time::UNIX_EPOCH,
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
    /// Should Sky print the conversation to stderr.
    #[arg(short)]
    print: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Set some runtime configuration, most importantly the Openai API KEY
    Config {
        /// Openai API KEY
        #[arg(short, long)]
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

trait Chat {
    fn say(&mut self, text: String) -> AIResponse;
}

#[derive(Debug)]
struct ChatWithAI {
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
    fn new(secrets: Config) -> Self {
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

struct ReportingToFile {
    chat: ChatWithAI,
    file: File,
}

impl ReportingToFile {
    fn new(chat: ChatWithAI, file: File) -> Self {
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

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Some(Command::Config { api_key, show }) => {
            if api_key.is_some() {
                confy::store("sky", None, Config { api_key })
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }

            if show {
                println!(
                    "{:?}",
                    confy::load::<Config>("sky", None)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                );
            }
        }
        None => {
            let cfg: Config =
                confy::load("sky", None).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let mut chat = chat_factory(cfg, args.print)?;

            prompt();
            for line in stdin().lines().flatten() {
                let response = chat.say(line);
                println!("\nSky: {response}\n");
                prompt();
            }
        }
    }

    Ok(())
}

#[inline(always)]
fn prompt() {
    print!("You: ");
    stdout().flush().ok();
}

#[inline]
fn chat_factory(cfg: Config, report: bool) -> io::Result<Box<dyn Chat>> {
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
