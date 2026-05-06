use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "patchwork", about = "AST-aware code editing with tree-sitter")]
pub struct Cli {
    #[arg(
        short = 'l',
        long = "language",
        global = true,
        help = "Force language (java, python, js, ts, tsx)"
    )]
    pub language: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print locations where code matches a pattern
    Find {
        #[arg(short = 'p', long = "pattern", help = "Code snippet to match")]
        pattern: Option<String>,
        #[arg(short = 'q', long = "query", help = "Tree-sitter query")]
        query: Option<String>,
        files: Vec<String>,
    },
    /// Replace matched code
    Replace {
        #[arg(short = 'p', long = "pattern")]
        pattern: Option<String>,
        #[arg(short = 'q', long = "query")]
        query: Option<String>,
        #[arg(short = 'r', long = "replacement")]
        replacement: String,
        #[arg(short = 'i', long = "in-place")]
        in_place: bool,
        files: Vec<String>,
    },
    /// Delete matched code
    Delete {
        #[arg(short = 'p', long = "pattern")]
        pattern: Option<String>,
        #[arg(short = 'q', long = "query")]
        query: Option<String>,
        #[arg(short = 'i', long = "in-place")]
        in_place: bool,
        files: Vec<String>,
    },
    /// Insert code before each match
    InsertBefore {
        #[arg(short = 'p', long = "pattern")]
        pattern: Option<String>,
        #[arg(short = 'q', long = "query")]
        query: Option<String>,
        #[arg(long = "code")]
        code: String,
        #[arg(short = 'i', long = "in-place")]
        in_place: bool,
        files: Vec<String>,
    },
    /// Insert code after each match
    InsertAfter {
        #[arg(short = 'p', long = "pattern")]
        pattern: Option<String>,
        #[arg(short = 'q', long = "query")]
        query: Option<String>,
        #[arg(long = "code")]
        code: String,
        #[arg(short = 'i', long = "in-place")]
        in_place: bool,
        files: Vec<String>,
    },
}

pub fn ensure_pattern_or_query(
    pattern: &Option<String>,
    query: &Option<String>,
) -> Result<(), String> {
    match (pattern, query) {
        (Some(_), Some(_)) => Err("Cannot use both --pattern (-p) and --query (-q)".into()),
        (None, None) => Err("Must provide either --pattern (-p) or --query (-q)".into()),
        _ => Ok(()),
    }
}
