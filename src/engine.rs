use std::time::Duration;
use serde::Serialize;

use libxml::parser::Parser;
use reqwest::{self, header, Client, ClientBuilder, Request, IntoUrl};
use thiserror::Error;
use url::Url;
use futures::channel::{mpsc, mpsc::UnboundedSender, oneshot};
use futures::stream::{self, StreamExt};
use futures::sink::SinkExt;

#[derive(Clone)]
pub struct Engine {
    client: Client,
    request_channel: UnboundedSender<(Request, oneshot::Sender<Result<String, EngineError>>)>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Parsing Error")]
    ParsingError(String),
    #[error("Network Error")]
    IoError(#[from] reqwest::Error),
}

pub struct Response {
    url: Url,
    body: String,
}

impl Response {
    /// Extract links using xpath guaranteeing them to be absolute.
    pub fn select_links(&self, xpath: &str) -> Result<Vec<Url>, EngineError> {
	let doc = libxml::parser::Parser::default_html()
            .parse_string(&self.body)
            .map_err(|e| EngineError::ParsingError(format!("{:}", e)))?;

	let root = doc.get_root_element().unwrap();
	let mut links = vec![];
	let nodes = root.findnodes(&xpath) 
            .map_err(|()| EngineError::ParsingError("Invalid XPath".to_string()))?;
	for node in nodes {
	    let link = node.get_content();
	    let link = self.url.join(&link)
		.map_err(|e| EngineError::ParsingError(format!("Invalid Url: {:}", e)))?;
	    links.push(link);
	}
	Ok(links)
    }

    pub fn extract_form_data(&self, xpath: &str) -> Result<Vec<(String, String)>, EngineError> {
        form_values(&self.body, xpath)
    }
}

async fn request2body(req: reqwest::Request, client: &Client) -> Result<String, EngineError> {
    let resp = client.execute(req).await?;
    let resp = resp.error_for_status()?;
    let body = resp.text().await?;
    Ok(body)
}
    
impl Engine {
    pub fn new() -> Result<Self, EngineError> {
	let mut headers = header::HeaderMap::new();
	headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("scrapers"),
	);
	let client = ClientBuilder::new()
            .default_headers(headers)
            .cookie_store(true)
            .timeout(Duration::from_secs(10))
            .build()?;
	let (tx, mut rv) = mpsc::unbounded::<(Request, oneshot::Sender<_>)>();

	let client_rt = client.clone();
	tokio::spawn(async move {
	    while let Some((req, back_tx)) = rv.next().await {
		let body = request2body(req, &client_rt).await;
		back_tx.send(body).expect("Failed sending to one-shot channel.");
	    }
	});

	Ok(Self {
	    client, request_channel: tx
	})
    }

    pub async fn get<T>(&mut self, url: T) -> Result<Response, EngineError>
	where T: IntoUrl + Clone + std::fmt::Display
    {
	let url = url.into_url()?;
	let retries: usize = 3;
	for _ in 0..retries {
	    let req = self.client.get(url.clone()).build()?;
	    let (tx, rx) = oneshot::channel();
	    self.request_channel.send((req, tx)).await.expect("Channel communication failed");

	    let body_res = rx.await.expect("Channel communication failed");
	    if let Err(e) = body_res {
		eprintln!("Error: {}; Retrying: {:}", e,  url);
		continue;
	    }
	    return Ok(body_res.map(|body| Response {
	    	body: body,
		url: url
	    })?);
	}
	unimplemented!();
    }
    
    pub async fn post<U, D>(&mut self, url: U, data: &D) -> Result<Response, EngineError>
    where D: Serialize,
	  U: IntoUrl + Clone + std::fmt::Display
    {
	let retries: usize = 3;
	let url = url.into_url()?;
	for _ in 0..retries {
	    let req = self.client.post(url.clone()).form(data).build()?;

	    dbg!(std::str::from_utf8(req.body().unwrap().as_bytes().unwrap()));
	    let (tx, rx) = oneshot::channel();
	    self.request_channel.send((req, tx)).await.expect("Channel communication failed");

	    let body_res = rx.await.expect("Channel communication failed");
	    // if let Err(e) = body_res {
	    // 	eprintln!("Error: {:}; Retrying: {:}", e,  url);
	    // 	continue;
	    // }
	    return Ok(body_res.map(|body| Response {
		url, body,
	    })?);
	}
	unimplemented!();
    }
}

// fn run_thing() {
//     let engine = Engine.default();
//     let search_request = {
// 	// Search params and stuff
// 	unimplemented!()
//     }
//     let item = engine.request(request);
// }

/// Extract pre-populated form data on a best-effort basis.
fn form_values(body: &str, form_xpath: &str) -> Result<Vec<(String, String)>, EngineError> {
    let parser = Parser::default_html();
    let doc = parser
        .parse_string(body)
        .map_err(|e| EngineError::ParsingError(format!("{:}", e)))?;
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
            if ["submit", "reset", "image"].contains(&t.as_str()) {
                continue;
            }
            if ["radio", "checkbox"].contains(&t.as_str()) {
                if node.get_attribute("checked") != Some("checked".to_string()) {
                    continue;
                }
            }
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
    use std::fs::read_to_string;
    use tokio;

    #[tokio::test]
    async fn parse_form() {
        let body = read_to_string("./src/test_data/DIP21.html").unwrap();
        let values = dbg!(form_values(&body, "//form").unwrap());
        assert_eq!(values.len(), 99);
    }

    #[tokio::test]
    async fn do_post() {
	/// The Url which has to be hit to set the cookies for subsequent search queries
	const COOKIE_LANDING: &str = "http://dipbt.bundestag.de/dip21.web/bt";

	let SEARCH_URL =
	    "http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do";

	// Start up the engine
	let mut engine = Engine::new().unwrap();
	// Get cookies
	engine.get(COOKIE_LANDING).await.unwrap();
	let resp = engine.get(SEARCH_URL).await.unwrap();

	let mut form = form_values(&resp.body, "//form").unwrap();

	// Add the parameters needed to kick of the search
	form.push(("method".to_string(), "Suchen".to_string()));

	let resp = engine.post(SEARCH_URL, &form).await.unwrap();
	for link in resp.select_links("//div[@class='tabelleGross']//a[@class='linkIntern']/@href").unwrap().into_iter() {
	    engine.get(link).await;
	}
    }    
}
