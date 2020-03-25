use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Field not set: {0}")]
    MissingField(&'static str),
    #[error("Scraping error")]
    ScrapingError(#[from] crate::scrapers::EngineError),
    #[error("Network Error")]
    IoError(#[from] reqwest::Error),    
}

#[derive(Debug)]
pub struct BipData {
    pub content: Option<String>,
    pub summary: String,
    pub tag_words: Option<String>,
}

pub struct BipDataBuilder {
    content: Option<String>,
    summary: Option<String>,
    tag_words: Option<String>,
}

impl BipData {
    pub fn builder() -> BipDataBuilder {
	BipDataBuilder {
	    content: None,
	    summary: None,
	    tag_words: None,
	}
    }
}

impl BipDataBuilder {
    pub fn content(self, input: Option<String>) -> Self {
 	Self {
	    content: input.map(|i| strip_whitespaces_and_linebreaks(&i)),
	    ..self
	}
    }
    pub fn summary(self, input: String) -> Self {
	Self {
	    summary: Some(strip_whitespaces_and_linebreaks(&input)),
	    ..self
	}
    }
    pub fn tag_words(self, input: Option<String>) -> Self {
	Self {
	    tag_words: input.map(|i| strip_whitespaces_and_linebreaks(&i)),
	    ..self
	}
    }

    pub fn build(self) -> Result<BipData, ParsingError> {
	let content = self.content; //.ok_or_else(|| ParsingError::MissingField("content"))?;
	let summary = self.summary.ok_or_else(|| ParsingError::MissingField("summary"))?;
	let tag_words = self.tag_words; //.ok_or_else(|| ParsingError::MissingField("tag_words"))?;
	Ok(BipData { content, summary, tag_words })
    }
}

// Strip whitespaces and multiple line breaks
fn strip_whitespaces_and_linebreaks(input: &str) -> String {
    let re_whitespace = Regex::new(r"[\t\v\f\r ]+").unwrap();
    let re_linebreaks = Regex::new(r"(\n\s*){2,}").unwrap();
    re_linebreaks
        .replace_all(
            &re_whitespace.replace_all(input, " "),
            "\n\n",
        )
        .to_string()
}
