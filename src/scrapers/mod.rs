use std::time::Duration;

use thiserror::Error;
use reqwest::{self, header, Client, ClientBuilder};

pub mod retry;
pub mod selector;
pub mod pipelines;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Parsing Error")]
    ParsingError(String),
    #[error("Network Error")]
    IoError(#[from] reqwest::Error),
    #[error("Requests where the body is a Stream cannot be clones")]
    RequestCloneError(String),
}

pub fn default_client() -> Result<Client, EngineError> {
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
    Ok(client)
}


#[cfg(test)]
mod tests {
    use futures::stream::{self, StreamExt, TryStreamExt};
    use reqwest::RequestBuilder;
    use tokio;
    use tower::{self, Service, ServiceBuilder};

    use crate::scrapers::{
	default_client,
	retry::RetryLimit,
	selector::Selector,
	pipelines::StdOutPipeline,
    };

    #[tokio::test]
    async fn do_post() {
        /// The Url which has to be hit to set the cookies for subsequent search queries
        const COOKIE_LANDING: &str = "http://dipbt.bundestag.de/dip21.web/bt";

        let search_url =
            "http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do";

        let client = default_client().unwrap();
        let svc = tower::service_fn(|req_builder: RequestBuilder| req_builder.send());

        let mut svc = ServiceBuilder::new()
            .rate_limit(10, std::time::Duration::from_secs(1))
            .retry(RetryLimit::new(3))
            .service(svc);

        let item_pipeline = StdOutPipeline;

        // Get cookies
        svc.call(client.get(COOKIE_LANDING)).await.unwrap();
        let resp = svc.call(client.get(search_url)).await.unwrap();
        let sel = Selector::from_response(resp).await.unwrap();

        let mut form = sel.form_data("//form").unwrap();
        // Add the parameters needed to kick off the search
        form.push(("method".to_string(), "Suchen".to_string()));

        loop {
            let req = client.post(search_url).form(&form);

            let resp = svc.call(req).await.unwrap();
            let sel = Selector::from_response(resp).await.unwrap();
            let links = sel
                .select_links("//div[@class='tabelleGross']//a[@class='linkIntern']/@href")
                .unwrap();
            let links = stream::iter(links)
                .then(|l| svc.call(client.get(dbg!(l))))
                .and_then(|resp| resp.text())
                .map_err(Into::into);
            item_pipeline.pipe_out(links).await;
            // Prepare the form for the next iteration
            form = sel.form_data("//form").unwrap();
            // Add the parameters needed to kick of the search
            form.push(("method".to_string(), ">".to_string()));
        }
    }
}
