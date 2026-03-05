//! ropt – Interactive CLI Option Configuration Tool
//!
//! Entry point: parse the CLI, dispatch to the appropriate handler, and
//! exit with a non-zero code on error so shell scripts can detect failures.

mod cli;
mod error;
mod executor;
mod node;
mod ops;
mod output;
mod renderer;
mod session;
mod show;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        // Print the full error chain to stderr.
        eprintln!("ropt: error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> error::Result<()> {
    match cli.command {
        // ── Session lifecycle ─────────────────────────────────────────────────
        Command::Begin => {
            let session_id = session::begin()?;
            println!("{session_id}");
        }

        Command::End(args) => {
            let session_id = session::resolve_session_id(args.session.as_deref())?;
            session::end(&session_id)?;
        }

        // ── Structure building ────────────────────────────────────────────────
        Command::Push(args) => {
            ops::push(&args)?;
        }

        Command::Append(args) => {
            ops::append(&args)?;
        }

        Command::Pop(args) => {
            let session_id = session::resolve_session_id(args.session.as_deref())?;
            ops::pop(&session_id)?;
        }

        // ── Execution & output ────────────────────────────────────────────────
        Command::Execute(args) => {
            let session_id = session::resolve_session_id(args.session.as_deref())?;
            executor::execute(&session_id)?;

            // After execution, read results back and print.
            let state = session::read_state(&session_id)?;
            output::write_results(&state.results, &args.format, &args.prefix)?;
        }

        Command::Read(args) => {
            let session_id = session::resolve_session_id(args.session.as_deref())?;
            let state = session::read_state(&session_id)?;

            let value = state.results.get(&args.key).ok_or_else(|| {
                anyhow::anyhow!(error::RoptError::ResultNotFound(args.key.clone()))
            })?;

            println!("{value}");
        }

        // ── Debugging ─────────────────────────────────────────────────────────
        Command::Show(args) => {
            let session_id = session::resolve_session_id(args.session.as_deref())?;
            show::show(&session_id, &args.format)?;
        }
    }

    Ok(())
}
