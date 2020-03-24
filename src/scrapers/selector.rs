use libxml::{parser::Parser, tree::Document};
use reqwest::Response;
use url::Url;

use crate::scrapers::EngineError;

pub struct Selector {
    base_url: Url,
    doc: Document,
}

impl Selector {
    pub fn new(base_url: Url, body: String) -> Result<Self, EngineError> {
        let doc = Parser::default_html()
            .parse_string(&body)
            .map_err(|e| EngineError::ParsingError(format!("{:}", e)))?;

        Ok(Self { base_url, doc })
    }

    pub async fn from_response(response: Response) -> Result<Self, EngineError> {
        let url = response.url().to_owned();
        let body = response.text().await?;
        Selector::new(url, body)
    }

    /// Extract links using xpath guaranteeing them to be absolute.
    pub fn select_links(&self, xpath: &str) -> Result<Vec<Url>, EngineError> {
        let root = self.doc.get_root_element().unwrap();
        let mut links = vec![];
        let nodes = root
            .findnodes(&xpath)
            .map_err(|()| EngineError::ParsingError("Invalid XPath".to_string()))?;
        for node in nodes {
            let link = node.get_content();
            let link = self
                .base_url
                .join(&link)
                .map_err(|e| EngineError::ParsingError(format!("Invalid Url: {:}", e)))?;
            links.push(link);
        }
        Ok(links)
    }

    pub fn form_data(&self, xpath: &str) -> Result<Vec<(String, String)>, EngineError> {
        form_values(&self.doc, xpath)
    }
}

/// Extract pre-populated form data on a best-effort basis.
fn form_values(doc: &Document, form_xpath: &str) -> Result<Vec<(String, String)>, EngineError> {
    let root = doc.get_root_element().expect("No root element found.");
    let form = root
        .findnodes(form_xpath)
        .map_err(|_: ()| EngineError::ParsingError("Invalid form XPath".to_string()))?
        .pop()
        .ok_or_else(|| EngineError::ParsingError("No form-node found".to_string()))?;
    let mut out = vec![];
    // input nodes
    for node in form.findnodes("descendant::input").unwrap() {
        if let Some(t) = node.get_attribute("type") {
            // Skip values which would be set on click
            if let "submit" | "reset" | "image" = t.as_str() {
                continue;
            }
            if let "radio" | "checkbox" = t.as_str() {
                if node.get_attribute("checked") != Some("checked".to_string()) {
                    continue;
                }
            };
        }
        if node.get_attribute("disabled") == Some("disabled".to_string()) {
            continue;
        }
        if let (Some(name), Some(value)) = (node.get_attribute("name"), node.get_attribute("value"))
        {
            out.push((name, value));
        }
    }
    // select nodes
    for node in form.findnodes("descendant::select").unwrap() {
        // multi-select is unsupported
        if let Some(name) = node.get_attribute("name") {
            let nodes: Vec<_> = node.findnodes("option").unwrap().into_iter().collect();
            // Get the first `selected` or if not found the simply the first node's value
            let value = nodes
                .iter()
                .filter(|n| n.get_attribute("selected") == Some("selected".to_string()))
                .chain(nodes.iter())
                .filter_map(|n| n.get_attribute("value"))
                .next();
            match value {
                None => eprintln!("select node had no usable child"),
                Some(value) => out.push((name, value)),
            };
        }
    }
    // text areas
    for node in form.findnodes("descendant::textarea").unwrap() {
        if let (Some(name), value) = (node.get_attribute("name"), node.get_content()) {
            out.push((name, value));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_form() {
        let doc = Parser::default_html()
            .parse_file("./src/test_data/DIP21.html")
            .unwrap();
        let values = dbg!(form_values(&doc, "//form").unwrap());
        assert_eq!(values.len(), 99);
    }
}
