mod config;
mod cppcheck;
mod fmtlogger;
mod issue;
mod result;

use std::{
    error::Error,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use crate::config::AnalyzerConfig;

const TOOLBOX_PATH: &str = "TOOLBOX_PATH";

fn run_cppcheck<P>(executable: &str, files: Vec<P>, output_path: &str)
where
    P: AsRef<Path>,
{
    let start = std::time::Instant::now();
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(&format!(
            "{} {} -j 6 --addon=misra --xml 2>{}",
            executable,
            files
                .iter()
                .map(|x| x.as_ref().display().to_string() + " ")
                .collect::<String>(),
            output_path
        ))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    log::debug!(
        "Running cppcheck START :: {:?} \n command: {:?}",
        start.elapsed(),
        command
    );
    let output = command.output();
    log::debug!("Ran cppcheck END :: {:?}", start.elapsed());
    log::trace!("{:#?}", output);
}

fn main() {
    // setup logging
    fmtlogger::default();

    // all errors are propagated to sentry with backtrace
    if let Err(err) = _main() {
        log::error!("error raised: {err}");
        // early exit with status 1
        process::exit(1);
    }
}

fn _main() -> Result<(), Box<dyn Error>> {
    let toolbox_directory = PathBuf::from(std::env::var(TOOLBOX_PATH).unwrap_or_else(|_| {
        log::warn!("`TOOLBOX_PATH` env not set, defaulting to `/toolbox`");
        "/toolbox".to_string()
    }));
    let cppcheck_executable = "cppcheck";
    let cppcheck_errors_path = toolbox_directory.join("cppcheck_error.xml");
    let analysis_config_path = toolbox_directory.join("analysis_config.json");
    let files_iter = std::fs::read_to_string(&analysis_config_path)
        .ok()
        .map(|s| serde_json::from_str::<AnalyzerConfig>(&s).ok())
        .flatten()
        .map_or_else(
            || {
                log::error!(
                    "Failed to load analysis config, at `{}`, using empty file list.",
                    analysis_config_path.display()
                );
                Vec::default()
            },
            AnalyzerConfig::cxx_files,
        );

    run_cppcheck(
        cppcheck_executable,
        files_iter,
        cppcheck_errors_path
            .to_str()
            .unwrap_or("/toolbox/cppcheck_error.xml"),
    );

    let mut issue_occurrences = vec![];
    if let Some(cppcheck_results) = std::fs::read_to_string(cppcheck_errors_path)
        .ok()
        .map(|src| quick_xml::de::from_str::<cppcheck::Results>(&src).unwrap())
    {
        log::debug!("{:?}", cppcheck_results);
        for error in cppcheck_results.errors.error {
            if let Some(issue_code) = cppcheck::mapping(&error.id) {
                let Some(location) = error.location.as_ref().and_then(|l| l.get(0)) else {
                    continue;
                };
                let issue_text = if error.msg.starts_with("misra") {
                    format!(
                        "{} {}",
                        error.id,
                        error.symbol.unwrap_or_else(|| "".to_string())
                    )
                } else {
                    error.msg
                };
                issue_occurrences.push(result::Issue {
                    issue_text,
                    issue_code,
                    location: result::Location {
                        path: location.file.clone(),
                        position: result::Position {
                            begin: result::Mark {
                                line: location.line,
                                column: location.column,
                            },
                            end: result::Mark {
                                line: location.line,
                                column: location.column,
                            },
                        },
                    },
                });
            }
        }
    }

    let json_output = serde_json::to_string(&issue_occurrences);
    log::debug!(
        "{}",
        json_output.as_ref().map(String::as_str).unwrap_or("{}")
    );

    let src = json_output.unwrap_or_else(|err| {
        log::error!("{err}");
        "{}".to_string()
    });

    let result_json = toolbox_directory.join("cppcheck_result.json");
    std::fs::write(result_json, src)?;

    Ok(())
}
