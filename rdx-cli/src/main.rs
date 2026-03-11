mod convert;
mod fmt;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rdx_parser::parse as parse_rdx;
use rdx_schema::{Schema, validate};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rdx")]
#[command(about = "Command-line interface for RDX documents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse an .rdx file and output the AST as JSON
    Parse {
        /// Path to the .rdx file
        file: PathBuf,

        /// Output formatted JSON
        #[arg(long)]
        pretty: bool,
    },

    /// Parse and validate against a schema
    Validate {
        /// Path to the .rdx file
        file: PathBuf,

        /// Path to the schema.json file
        #[arg(long)]
        schema: PathBuf,
    },

    /// Convert .mdx to .rdx
    Convert {
        /// Path to the .mdx file
        file: PathBuf,

        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Write to same path but with .rdx extension
        #[arg(long)]
        in_place: bool,
    },

    /// Format an .rdx file
    Fmt {
        /// Path to the .rdx file
        file: PathBuf,

        /// Write formatted output back to the file
        #[arg(long)]
        write: bool,

        /// Check if the file is already formatted (exit 1 if not)
        #[arg(long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file, pretty } => {
            cmd_parse(&file, pretty)?;
        }
        Commands::Validate { file, schema } => {
            cmd_validate(&file, &schema)?;
        }
        Commands::Convert {
            file,
            output,
            in_place,
        } => {
            cmd_convert(&file, output, in_place)?;
        }
        Commands::Fmt { file, write, check } => {
            cmd_fmt(&file, write, check)?;
        }
    }

    Ok(())
}

/// Parse an .rdx file and output the AST as JSON
fn cmd_parse(file: &PathBuf, pretty: bool) -> Result<()> {
    let content =
        fs::read_to_string(file).with_context(|| format!("Failed to read file: {:?}", file))?;

    let root = parse_rdx(&content);

    // Serialize to JSON
    let json_value = serde_json::to_value(&root).context("Failed to serialize AST to JSON")?;

    let output = if pretty {
        serde_json::to_string_pretty(&json_value).context("Failed to format JSON")?
    } else {
        serde_json::to_string(&json_value).context("Failed to format JSON")?
    };

    println!("{}", output);
    Ok(())
}

/// Parse and validate against a schema
fn cmd_validate(file: &PathBuf, schema: &PathBuf) -> Result<()> {
    let content =
        fs::read_to_string(file).with_context(|| format!("Failed to read file: {:?}", file))?;

    let schema_content = fs::read_to_string(schema)
        .with_context(|| format!("Failed to read schema file: {:?}", schema))?;

    // Parse the RDX document
    let root = parse_rdx(&content);

    // Parse the schema from JSON
    let schema: Schema =
        serde_json::from_str(&schema_content).context("Failed to parse schema JSON")?;

    // Validate
    let diagnostics = validate(&root, &schema);

    // Print diagnostics to stderr
    let mut has_errors = false;
    for diagnostic in &diagnostics {
        eprintln!("{:?}", diagnostic);
        if diagnostic.severity == rdx_schema::Severity::Error {
            has_errors = true;
        }
    }

    // Exit with error code if there were errors
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Convert .mdx to .rdx
fn cmd_convert(file: &PathBuf, output: Option<PathBuf>, in_place: bool) -> Result<()> {
    let content =
        fs::read_to_string(file).with_context(|| format!("Failed to read file: {:?}", file))?;

    let (converted, warnings) = convert::convert_mdx_to_rdx(&content);

    // Print warnings to stderr
    for warning in warnings {
        eprintln!(
            "Warning (line {}): {}",
            warning.line_number, warning.message
        );
    }

    // Determine output path
    let output_path = if in_place {
        let mut new_path = file.clone();
        new_path.set_extension("rdx");
        new_path
    } else if let Some(out) = output {
        out
    } else {
        // Output to stdout if no output path specified
        println!("{}", converted);
        return Ok(());
    };

    // Write to file
    fs::write(&output_path, &converted)
        .with_context(|| format!("Failed to write to file: {:?}", output_path))?;

    eprintln!("Converted to: {:?}", output_path);
    Ok(())
}

/// Format an .rdx file
fn cmd_fmt(file: &PathBuf, write: bool, check: bool) -> Result<()> {
    let content =
        fs::read_to_string(file).with_context(|| format!("Failed to read file: {:?}", file))?;

    let root = parse_rdx(&content);
    let formatted = fmt::format_root(&root);

    if check {
        if content != formatted {
            eprintln!("{:?} is not formatted", file);
            std::process::exit(1);
        }
        return Ok(());
    }

    if write {
        fs::write(file, &formatted).with_context(|| format!("Failed to write file: {:?}", file))?;
        eprintln!("Formatted: {:?}", file);
    } else {
        print!("{}", formatted);
    }

    Ok(())
}
