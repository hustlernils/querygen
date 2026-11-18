use std::fmt;
use std::path::PathBuf;
use std::process::exit;

use chumsky::Parser as ChumskyParser;
use chumsky::error::Rich;
use clap::{Parser, ValueEnum};
use querygen::generation::{AcmDl, IeeeXplore, QueryGenerator};
use querygen::parser::program;

#[derive(Clone, Debug, PartialEq, ValueEnum)]
enum TargetDatabase {
    Acm,
    Ieee,
}

impl TargetDatabase {
    fn generator(&self) -> &'static dyn QueryGenerator {
        match self {
            TargetDatabase::Acm => &AcmDl,
            TargetDatabase::Ieee => &IeeeXplore
        }
    }
}

impl fmt::Display for TargetDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    query: PathBuf,

    #[arg(long = "target", value_delimiter = ',', required = true)]
    targets: Vec<TargetDatabase>,

    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn compile<'a>(
    targets: &[TargetDatabase],
    source: &'a str,
) -> Result<String, Vec<Rich<'a, char>>> {
    let ast = program().parse(source).into_result()?;

    let mut out = String::new();
    for (i, target) in targets.iter().enumerate() {
        if targets.len() > 1 {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!("# {target}\n"));
        }
        out.push_str(&target.generator().generate(&ast));
        out.push('\n');
    }
    Ok(out)
}

fn dedup(targets: Vec<TargetDatabase>) -> Vec<TargetDatabase> {
    let mut unique = Vec::new();
    for target in targets {
        if !unique.contains(&target) {
            unique.push(target);
        }
    }
    unique
}

fn main() {
    let args = Args::parse();

    let source = match std::fs::read_to_string(&args.query) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", args.query.display());
            exit(1);
        }
    };

    let targets = dedup(args.targets);

    let output = match compile(&targets, &source) {
        Ok(output) => output,
        Err(errors) => {
            // TODO: replace with ariadne reports
            for e in errors {
                eprintln!("error at {:?}: {e}", e.span());
            }
            exit(1);
        }
    };

    match args.output {
        None => print!("{output}"),
        Some(path) => {
            if let Err(e) = std::fs::write(&path, output) {
                eprintln!("error: cannot write {}: {e}", path.display());
                exit(1);
            }
        }
    }
}
