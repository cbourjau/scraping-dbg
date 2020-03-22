use std::fs;
use std::io::Error;
use std::process::Command;

use tempfile::NamedTempFile;

/// Open the given string in the default browser. This does not block the executor.
pub fn open_in_browser(html: &str) -> Result<(), Error> {
    let (_file, pathbuf) = NamedTempFile::new()?.keep()?;
    fs::write(&pathbuf, html)?;
    let pstr = format!("{}.html", pathbuf.to_str().unwrap());
    fs::rename(&pathbuf, &pstr)?;
    Command::new("xdg-open").arg(pstr).output()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn open_browser() {
        open_in_browser("Hello world!").unwrap();
    }
}
