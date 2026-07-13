use anyhow::{Context, Result};
use idud_hygiene::{enforce_golden_pattern, report_golden_pattern};
use std::{env, process};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut report = false;
    let mut repo_root = None;
    let mut manifest_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" => report = true,
            _ if repo_root.is_none() => repo_root = Some(arg),
            _ if manifest_path.is_none() => manifest_path = Some(arg),
            _ => {
                return Err(anyhow::anyhow!(
                    "unexpected argument {arg}; usage: idud-hygiene [--report] <repo-root> [manifest-path]"
                ))
            }
        }
    }

    let repo_root = repo_root.context("usage: idud-hygiene [--report] <repo-root> [manifest-path]")?;
    let manifest_path = manifest_path.unwrap_or_else(|| "golden_patterns/architecture_hygiene.json".to_string());

    if report {
        let reports = report_golden_pattern(&repo_root, &manifest_path)?;
        for report in reports {
            let status = if report.passed { "PASS" } else { "FAIL" };
            println!("[{status}] {}", report.id);
            for violation in report.violations {
                println!("  - {violation}");
            }
        }
        return Ok(());
    }

    let violations = enforce_golden_pattern(&repo_root, &manifest_path)?;
    if violations.is_empty() {
        println!("No hygiene violations found.");
        Ok(())
    } else {
        eprintln!("Hygiene violations detected:\n{}", violations.join("\n"));
        process::exit(1);
    }
}
