use crate::ApiError;
use libxml::{
    parser::Parser,
    tree::document::Document
};
use regex::Regex;

#[derive(Debug)]
pub struct BipData {
    pub content: String,
    pub summary: String,
    pub tag_words: String,
}

impl BipData {
    pub fn from_html(html: &str) -> Result<Self, ApiError> {
        let doc = Parser::default_html()
            .parse_string(&html)
            .map_err(|e| ApiError::ParsingError(e.to_string()))?;
        // let result = "Hello World!", "x").to_string();
        fn extract_and_strip(doc: &Document, xpath: &str) -> Result<String, ApiError>{
            let re_whitespace = Regex::new(r"[\t\v\f\r ]+").unwrap();
            let re_linebreaks = Regex::new(r"(\n\s*){2,}").unwrap();
            let root = doc
		.get_root_element()
		.ok_or_else(|| ApiError::ParsingError("No root Element.".to_string()))?;
	    
            let nodes = root
                .findnodes(xpath)
                .map_err(|_: ()| ApiError::ParsingError("Failed to apply xpath".to_string()))?;
            match nodes.as_slice() {
                [node] => Ok(re_linebreaks
                    .replace_all(
                        &re_whitespace.replace_all(&doc.node_to_string(node), " "),
                        "\n\n",
                    )
                    .to_string()),
                _ => {
                    Err(ApiError::ParsingError("Unexpected HTML schema".to_string()))
                }
            }
        };

        let content = extract_and_strip(&doc, "//fieldset[h1[contains(text(), 'Inhalt')]]")?;
        let summary = extract_and_strip(&doc, "//fieldset[h1[contains(text(), 'Basisinformationen')]]")?;
        let tag_words = extract_and_strip(&doc, "//fieldset[h1[contains(text(), 'Schlagwörter')]]")?;
        Ok(BipData {
            content,
            summary,
            tag_words,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BipClient, ElectionPeriod};
    use tokio;

    use futures::stream::StreamExt;

    #[tokio::test]
    async fn fetch_details_and_parse() {
        let client = BipClient::new().await.unwrap();
        let mut details_stream = client.detail_views(2019, ElectionPeriod::EP18).boxed();
        let mut cnt = 0;
        while let Some(Ok(html)) = details_stream.next().await {
            BipData::from_html(&html).unwrap();
            cnt += 1;
        }
        assert_eq!(cnt, 14)
    }
}
