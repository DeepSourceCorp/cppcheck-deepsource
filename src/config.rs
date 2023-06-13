use std::path::PathBuf;

use serde::Deserialize;

#[derive(Default, Deserialize, Debug)]
pub struct AnalyzerConfig {
    files: Vec<PathBuf>,
    #[serde(default)]
    pub analyzer_meta: AnalyzerMeta,
}

impl AnalyzerConfig {
    pub fn cxx_files(self) -> Vec<PathBuf> {
        self.files
            .into_iter()
            .filter(|f| !f.is_symlink())
            .filter(|f| f.is_file())
            .filter(|f| {
                f.extension()
                    .map(|x| x.eq("cpp") | x.eq("c"))
                    .unwrap_or_default()
            })
            // ignore files > ~25MB in size
            .filter(|f| {
                !f.metadata()
                    .map(|x| x.len() > 25_000_000)
                    .unwrap_or_default()
            })
            .collect()
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct AnalyzerMeta {
    pub name: String,
    pub enabled: bool,
    // todo(swarnim): add misra_compliance: bool
}
