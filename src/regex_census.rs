use std::io::{BufWriter, Write};
use std::path::PathBuf;
use syntect::parsing::SyntaxSet;

pub(crate) struct Capture<'a> {
    syntaxes: &'a SyntaxSet,
    path: Option<PathBuf>,
}

impl<'a> Capture<'a> {
    pub(crate) fn new(syntaxes: &'a SyntaxSet) -> Self {
        Self {
            syntaxes,
            path: std::env::var_os("NATIVE_DIFF_REGEX_CENSUS").map(PathBuf::from),
        }
    }

    fn write(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let mut output = BufWriter::new(std::fs::File::create_new(path)?);
        serde_json::to_writer(&mut output, &self.syntaxes.regex_census())?;
        output.write_all(b"\n")?;
        output.flush()?;
        Ok(())
    }
}

impl Drop for Capture<'_> {
    fn drop(&mut self) {
        if let Some(path) = &self.path
            && let Err(error) = self.write(path)
        {
            eprintln!("Regex census failed: {error:#}");
        }
    }
}
