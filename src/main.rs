mod config;
mod cppcheck;
mod fmtlogger;
mod issue;
mod result;

use std::{
    collections::HashSet,
    error::Error,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

use crate::config::AnalyzerConfig;

use env_struct::env_struct;
env_struct! {
    #[derive(Debug)]
    pub struct Env {
        pub toolbox_path = "/toolbox".into(),
        pub code_path = "/code".into(),
    }
}

env_struct! {
    #[derive(Debug)]
    pub struct CppcheckEnv {
        pub cppcheck_cache_path,
    }
}

fn run_cppcheck<'a>(executable: &str, code_path: impl AsRef<str>, output_path: impl AsRef<Path>) {
    let start = std::time::Instant::now();
    // ensure the command is run in `sh`
    let mut command_with_sh = Command::new("sh");
    // only enable caching if cache_path is set
    let cppcheck_build_dir = if let Ok(cppcheck_env) = CppcheckEnv::try_load_from_env() {
        format!("--cppcheck-build-dir={}", cppcheck_env.cppcheck_cache_path)
    } else {
        "".to_string()
    };
    let code_dir = code_path.as_ref();
    let output_file = output_path.as_ref().display();
    // build the command to run
    command_with_sh
        .arg("-c")
        .arg(format!("{executable} {code_dir} -l 6 --std=c++20 --addon=misra --xml --output-file={output_file} {cppcheck_build_dir}"))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    log::debug!("Running cppcheck START :: {:?}", start.elapsed());
    log::debug!("Shell command: {command_with_sh:?}");
    let output = command_with_sh.output();
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
    let env = Env::load_from_env();
    let toolbox_directory = PathBuf::from(env.toolbox_path);
    let cppcheck_executable = "cppcheck";
    let cppcheck_output_path = toolbox_directory.join("cppcheck_error.xml");
    let analysis_config_path = toolbox_directory.join("analysis_config.json");
    let files_set: HashSet<PathBuf> = HashSet::from_iter(
        std::fs::read_to_string(&analysis_config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<AnalyzerConfig>(&s).ok())
            .map_or_else(
                || {
                    log::error!(
                        "Failed to load analysis config, at `{}`, using empty file list.",
                        analysis_config_path.display()
                    );
                    Vec::default()
                },
                AnalyzerConfig::cxx_files,
            ),
    );

    run_cppcheck(cppcheck_executable, env.code_path, &cppcheck_output_path);

    log::debug!("{:#?}", files_set);
    let mut issue_occurrences = vec![];
    if let Some(cppcheck_results) = std::fs::read_to_string(cppcheck_output_path)
        .ok()
        .map(|src| quick_xml::de::from_str::<cppcheck::Results>(&src).unwrap())
    {
        // log::debug!("{:?}", cppcheck_results);
        for error in cppcheck_results.errors.error {
            if let Some(issue_code) = cppcheck::mapping(&error.id) {
                let Some(location) = error.location.as_ref().and_then(|l| l.get(0)) else {
                    continue;
                };
                if !files_set.contains(&PathBuf::from(&location.file)) {
                    continue;
                }
                let issue_text = if error.msg.starts_with("misra") {
                    format!(
                        "{} {}",
                        error.id,
                        error
                            .symbol
                            .get(0)
                            .map(String::as_str)
                            .unwrap_or_else(|| "")
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
