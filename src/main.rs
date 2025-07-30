//! # hayatmoji-cli
//! A hayatmoji interactive cli tool for using emojis on commits. 🤦

use std::{error::Error, fmt::Display};

use clap::{CommandFactory, Parser};
use dialoguer::{FuzzySelect, Input, theme::ColorfulTheme};
use git2::Repository;
use once_cell::sync::Lazy;
use serde::Deserialize;

const HAYATMOJIS_TOML: &str = include_str!("../hayatmoji/hayatmojis.toml");

static HAYATMOJIS: Lazy<Vec<Hayatmoji>> = Lazy::new(|| {
    let root: toml::Value = toml::from_str(HAYATMOJIS_TOML).unwrap();
    let table = root["hayatmojis"].as_table().unwrap();
    table
        .values()
        .map(|value| value.clone().try_into().unwrap())
        .collect()
});

#[derive(Deserialize)]
struct Hayatmoji {
    emoji: String,
    background: String,
    description: String,
}

impl Display for Hayatmoji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.emoji, self.description)
    }
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}

#[derive(Parser)]
#[command(about = "A hayatmoji client for using emojis on commit messages.")]
struct Args {
    #[arg(short, long, help = "Interactively commit using the prompts")]
    commit: bool,
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    if args.commit {
        // dialoguer operation
        let hayatmoji = ask_hayatmoji("Choose a hayatmoji")?;
        let title = ask_title("Enter the commit title")?;
        let message = ask_message("Enter the commit message")?;

        // git operation
        let repo = Repository::open(".")?;
        ensure_staged(&repo)?;
        commit(
            &repo,
            &if let Some(message) = message {
                format!("{} {}\n\n{}", hayatmoji, title, message)
            } else {
                format!("{} {}", hayatmoji, title)
            },
        )?;
    } else {
        Args::command().print_help()?;
    }
    Ok(())
}

fn ensure_staged(repo: &Repository) -> Result<(), git2::Error> {
    let statuses = repo.statuses(None)?;
    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
            || status.is_index_typechange()
        {
            return Ok(());
        }
    }
    Err(git2::Error::from_str("nothing to commit"))
}

fn commit(repo: &Repository, message: &str) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.write()?;
    let new_tree_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_tree_oid)?;
    let author = repo.signature()?;
    let head = repo.head()?;
    let parent = repo.find_commit(
        head.target()
            .ok_or(git2::Error::from_str("failed to get the OID"))?,
    )?;
    repo.commit(
        Some("HEAD"),
        &author,
        &author,
        message,
        &new_tree,
        &[&parent],
    )?;
    Ok(())
}

fn ask_hayatmoji(prompt: &str) -> Result<String, dialoguer::Error> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&HAYATMOJIS)
        .default(0)
        .max_length(6)
        .interact()?;
    Ok(HAYATMOJIS[selection].emoji.clone())
}

fn ask_title(prompt: &str) -> Result<String, dialoguer::Error> {
    let res = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .allow_empty(false)
        .interact_text()?;
    Ok(res)
}

fn ask_message(prompt: &str) -> Result<Option<String>, dialoguer::Error> {
    let res = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;
    Ok(Some(res).filter(|s| !s.is_empty()))
}
