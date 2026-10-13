use anyhow::Result;

pub(crate) fn finish<T>(operation: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (operation, cleanup) {
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("Cleanup also failed: {cleanup:#}")))
        }
        (result, Ok(())) => result,
        (Ok(_), Err(error)) => Err(error),
    }
}
