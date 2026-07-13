use anyhow::{Context, Result};
use idud_hygiene::{enforce_golden_manifests, render_hygiene_dashboard, report_golden_manifests};
use serde_json::json;
use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{self, Command},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        process::exit(1);
    }
}

fn open_in_browser(path: &Path) -> Result<()> {
    let path_display = path.to_string_lossy().to_string();
    let mut attempts: Vec<(&str, Vec<&str>)> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        attempts.push(("cmd", vec!["/c", "start", "", path_display.as_str()]));
    }

    #[cfg(target_os = "macos")]
    {
        attempts.push(("open", vec![path_display.as_str()]));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        attempts.push(("xdg-open", vec![path_display.as_str()]));
        attempts.push(("gio", vec!["open", path_display.as_str()]));
    }

    let mut errors = Vec::new();
    for (program, args) in attempts {
        match Command::new(program).args(args).spawn() {
            Ok(_) => return Ok(()),
            Err(err) => errors.push(format!("{program}: {err}")),
        }
    }

    Err(anyhow::anyhow!(
        "failed to open the report in a browser; tried {}",
        errors.join(", ")
    ))
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut report = false;
    let mut html = false;
    let mut json_output = false;
    let mut manifest_stdin = false;
    let mut should_open_in_browser = false;
    let mut output_path = None;
    let mut repo_root = None;
    let mut manifest_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" | "-report" => report = true,
            "--html" | "-html" => html = true,
            "--json" | "-json" => json_output = true,
            "--manifest-stdin" => manifest_stdin = true,
            "--open" | "-open" => should_open_in_browser = true,
            "--output" | "-output" | "-o" => {
                let Some(path) = args.next() else {
                    return Err(anyhow::anyhow!("--output requires a file path"));
                };
                output_path = Some(path);
            }
            _ if arg.starts_with('-') => {
                return Err(anyhow::anyhow!(
                    "unexpected argument {arg}; usage: idud-hygiene [--report|--html] [--json] [--manifest-stdin] [--open] [--output PATH] <repo-root> [manifest-path]"
                ))
            }
            _ if repo_root.is_none() => repo_root = Some(arg),
            _ if manifest_path.is_none() => manifest_path = Some(arg),
            _ => {
                return Err(anyhow::anyhow!(
                    "unexpected argument {arg}; usage: idud-hygiene [--report|--html] [--json] [--manifest-stdin] [--open] [--output PATH] <repo-root> [manifest-path]"
                ))
            }
        }
    }

    if report && html {
        return Err(anyhow::anyhow!("choose either --report or --html"));
    }
    if html && json_output {
        return Err(anyhow::anyhow!("choose either --html or --json"));
    }

    let repo_root =
        repo_root.context("usage: idud-hygiene [--report|--html] [--json] [--manifest-stdin] [--open] [--output PATH] <repo-root> [manifest-path]")?;
    let manifest_path = if manifest_stdin {
        let mut manifest_content = String::new();
        io::stdin()
            .read_to_string(&mut manifest_content)
            .context("failed to read manifest JSON from stdin")?;
        let temp_path = env::temp_dir().join(format!("idud-hygiene-manifest-{}.json", process::id()));
        fs::write(&temp_path, manifest_content)
            .with_context(|| format!("failed to write stdin manifest to {}", temp_path.display()))?;
        temp_path.to_string_lossy().to_string()
    } else {
        manifest_path.unwrap_or_else(|| "crates/idud-hygiene/golden_patterns".to_string())
    };

    if html {
        let dashboard = render_hygiene_dashboard(&repo_root, &manifest_path)?;
        let target_path = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            let temp_dir = env::temp_dir();
            let file_name = format!("idud-hygiene-dashboard-{}.html", process::id());
            temp_dir.join(file_name)
        };

        fs::write(&target_path, dashboard)
            .with_context(|| format!("failed to write HTML report to {}", target_path.display()))?;
        println!("Wrote HTML hygiene dashboard to {}", target_path.display());

        if should_open_in_browser {
            open_in_browser(&target_path)?;
            println!("Opened the report in your default browser.");
        }
        return Ok(());
    }

    if report {
        let manifests = report_golden_manifests(&repo_root, &manifest_path)?;
        if json_output {
            println!("{}", serde_json::to_string_pretty(&manifests)?);
            return Ok(());
        }
        for manifest in manifests {
            println!(
                "[{}] {} ({})",
                if manifest.passed { "PASS" } else { "FAIL" },
                manifest.name,
                manifest.path
            );
            for report in manifest.rules {
                let status = if report.passed { "PASS" } else { "FAIL" };
                println!("  [{status}] {}", report.id);
                for violation in report.violations {
                    println!("    - {violation}");
                }
            }
        }
        return Ok(());
    }

    let violations = enforce_golden_manifests(&repo_root, &manifest_path)?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "passed": violations.is_empty(),
                "violations": violations
            }))?
        );
        if violations.is_empty() {
            Ok(())
        } else {
            process::exit(1);
        }
    } else if violations.is_empty() {
        println!("No hygiene violations found.");
        Ok(())
    } else {
        eprintln!("Hygiene violations detected:\n{}", violations.join("\n"));
        process::exit(1);
    }
}
