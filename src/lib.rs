use std::collections::HashMap;
use std::time::Duration;

use futures::stream::{iter, once, Stream, StreamExt};
use libxml;
use reqwest::header;
use reqwest::{self, Client, ClientBuilder};
use thiserror::Error;
use url::Url;

pub mod parsing;
pub mod engine;

#[derive(Debug, PartialEq)]
pub enum Citation {
    BGBl1 { year: i32, page: u32 },
    BGBl2 { year: i32, page: u32 },
}

/// The Url which has to be hit to set the cookies for subsequent search queries
const COOKIE_LANDING: &str = "http://dipbt.bundestag.de/dip21.web/bt";

/// Url for search queries. Requires that the right cookies are present in the header
const SEARCH_URL: &str =
    "http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do";

/// Base Url for relative links
const BASE_URL: &str = "http://dipbt.bundestag.de/";

const PARALLEL_DOWNLOADS: usize = 1;

#[derive(Clone, Debug)]
pub struct BipClient {
    /// Client with necessary cookies set
    cookied_client: Client,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Network Error")]
    IoError(#[from] reqwest::Error),
    #[error("Failed to get session cookie")]
    NoCookie,
    #[error("Parsing Error")]
    ParsingError(String),
    #[error("Session was closed by host")]
    LoggedOut,
    #[error("Failed to select links: {0}")]
    SelectionError(String),
}

#[derive(Copy, Clone, Debug)]
pub enum ElectionPeriod {
    EP7 = 17,
    EP8 = 16,
    EP9 = 15,
    EP10 = 14,
    EP11 = 13,
    EP12 = 12,
    EP13 = 11,
    EP14 = 10,
    EP15 = 9,
    EP16 = 4,
    EP17 = 5,
    EP18 = 6,
    EP19 = 8,
}

/// Construct a default client with the correct USER_AGENT and
/// timeouts set.
/// This client is sufficient for accessing the details page, but not
/// for performing queries. For the latter, construct a `BipClient`.
fn default_client() -> Result<Client, ApiError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("librelaws"),
    );

    Ok(ClientBuilder::new()
        .default_headers(headers)
        .cookie_store(true)
        .timeout(Duration::from_secs(20))
        .build()?)
}

impl BipClient {
    /// Create a client which has the necessary cookies set for subsequent queries
    pub async fn new() -> Result<Self, ApiError> {
        let cookied_client = default_client()?;
        let resp = cookied_client
            .get(COOKIE_LANDING)
            .send()
            .await?;

        // Make sure that we actually got a cookie!
        if resp.cookies().count() == 0 {
            return Err(ApiError::NoCookie);
        }
        Ok(Self { cookied_client })
    }

    /// Stream over detail view pages for the given year and period
    pub fn detail_views<'a>(
        &'a self,
        year: u32,
        period: ElectionPeriod,
    ) -> impl Stream<Item = Result<String, ApiError>> + 'a {
        let links =
            once(async move { iter(self.list_view(year, period).await.unwrap()) }).flatten();
        links
            .map(move |l| {
                async move {
                    Ok(self.cookied_client
                        .get(l)
                        .send()
                        .await?
                        .text() // _with_charset("ISO-8859-15")
                        .await?)
                }
            })
            .buffered(PARALLEL_DOWNLOADS)
    }

    /// Scrape all links to detail_pages for the given year and election period.
    async fn list_view(&self, year: u32, period: ElectionPeriod) -> Result<Vec<Url>, ApiError> {
        let mut links = vec![];
        let mut offset = 0;
        loop {
            let page_ids = self.list_view_paged(offset, year, period).await?;
            if page_ids.is_empty() {
                break;
            }
            links.extend(page_ids.into_iter());
            offset += 100;
        }
        Ok(links)
    }

    async fn list_view_paged(
        &self,
        offset: u32,
        year: u32,
        period: ElectionPeriod,
    ) -> Result<Vec<Url>, ApiError> {
        // let name = "BGBl I";
        let year = year.to_string();
        let offset_string = offset.to_string();
        let period = (period as i32).to_string();
        let query = {
            let mut q = default_query();
            // q.insert("verkuendungsblatt", name).unwrap();
            q.insert("jahrgang", &year).unwrap();
            q.insert("offset", &offset_string);
            q.insert("wahlperiode", &period);
            q
        };
        let n_retries = 3;
        let mut i_try = 0;

        loop {
            let resp = self
                .cookied_client
                .post(SEARCH_URL)
                .form(&query)
                .send()
                .await?;
            match resp.error_for_status() {
                Ok(resp) => {
                    let body = resp.text().await?;
                    if body.contains("Sie wurden vom System abgemeldet") {
                        return Err(ApiError::LoggedOut);
                    }
                    return detail_links(&body);
                }
                Err(e) => {
                    if i_try < n_retries {
                        dbg!(i_try += 1);
                        continue;
                    } else {
                        return Err(ApiError::IoError(e));
                    }
                }
            }
        }
    }
}

/// Prepare the query parameters as expected by bip
fn default_query<'a>() -> HashMap<&'a str, &'a str> {
    [
        ("verkuendungsblatt", ""), // {BGBl I, BGBl II}
        ("jahrgang", ""),
        ("seite", ""),
        ("wahlperiode", ""),
        // Unused parameters which are expected by the server...
        ("drsId", ""),
        ("plprId", ""),
        ("aeDrsId", ""),
        ("aePlprId", ""),
        ("vorgangId", ""),
        ("procedureContext", ""),
        ("vpId", ""),
        ("formChanged", "false"),
        ("promptUser", "false"),
        ("overrideChanged", "true"),
        ("javascriptActive", "yes"),
        ("personId", ""),
        ("personNachname", ""),
        ("prompt", "no"),
        ("anchor", ""),
        ("wahlperiodeaktualisiert", "false"),
        ("startDatum", ""),
        ("endDatum", ""),
        ("includeVorgangstyp", "UND"),
        ("nummer", ""),
        ("suchwort", ""),
        ("suchwortUndSchlagwort", "ODER"),
        ("schlagwort1", ""),
        ("linkSchlagwort2", "UND"),
        ("schlagwort2", ""),
        ("linkSchlagwort3", "UND"),
        ("schlagwort3", ""),
        ("unterbegriffsTiefe", "0"),
        ("sachgebiet", ""),
        ("includeKu", "UND"),
        ("ressort", ""),
        ("nachname", ""),
        ("vorname", ""),
        ("heftnummer", ""),
        ("verkuendungStartDatum", ""),
        ("verkuendungEndDatum", ""),
        ("btBrBeteiligung", "alle"),
        ("gestaOrdnungsnummer", ""),
        ("beratungsstand", ""),
        ("signaturParlamentsarchiv", ""),
        ("method", "Suchen"),
    ]
    .iter()
    .map(|el| el.to_owned())
    .collect::<HashMap<_, _>>()
}

fn detail_links(body: &str) -> Result<Vec<Url>, ApiError> {
    let doc = libxml::parser::Parser::default_html()
        .parse_string(&body)
        .map_err(|e| ApiError::SelectionError(format!("{:}", e)))?;

    let root = doc.get_root_element().unwrap();
    let mut links = vec![];
    let nodes = root.findnodes("//div[@class='tabelleGross']//a[@class='linkIntern']/@href") 
        .map_err(|()| ApiError::SelectionError("Invalid XPath".to_string()))?;
    for node in nodes {
	let link = node.get_content();
	let link = Url::parse(BASE_URL)
	    .and_then(|base| base.join(&link))
	    .map_err(|e| ApiError::SelectionError(format!("Invalid Url: {:}", e)))?;
	links.push(link);
    }
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::fs;
    use tokio;

    #[tokio::test]
    async fn fetch_list() {
        let client = BipClient::new().await.unwrap();
        dbg!(client.list_view(2019, ElectionPeriod::EP18).await.unwrap());
    }

    #[tokio::test]
    async fn fetch_details() {
        let client = BipClient::new().await.unwrap();
        let mut details_stream = client.detail_views(2019, ElectionPeriod::EP18).boxed();
        while let Some(s) = details_stream.next().await {
            dbg!(&s.unwrap()[..1000]);
        }
    }
}
