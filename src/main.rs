use clap::{Parser, Subcommand};
use sky::{config::Config, *};
use std::{
    error::Error,
    io::{stdin, stdout, Write},
};

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

fn main() -> std::result::Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    match args.command {
        Some(Command::Config { api_key, show }) => {
            if api_key.is_some() {
                confy::store("sky", None, Config { api_key })?
            }

            if show {
                println!("{:?}", confy::load::<Config>("sky", None)?);
            }
        }
        None => {
            let cfg: Config = confy::load("sky", None)?;

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
