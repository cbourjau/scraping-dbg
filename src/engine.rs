use std::time::Duration;

use async_trait::async_trait;
use reqwest::{self, header, Client, ClientBuilder, RequestBuilder, Response};

use futures::future::TryFutureExt;
use futures::stream::{Stream, StreamExt};

use crate::EngineError;

#[async_trait]
trait ExecuteRetry {
    async fn send_retry(self, retries: usize) -> Result<Response, EngineError>;
}

#[async_trait]
impl ExecuteRetry for RequestBuilder {
    async fn send_retry(self, retries: usize) -> Result<Response, EngineError> {
        for _ in 0..retries {
            let request_cl = dbg!(&self).try_clone().ok_or_else(|| {
                EngineError::RequestCloneError("Failed to clone request".to_string())
            })?;
            let resp = request_cl
                .send()
                .and_then(|resp| async { resp.error_for_status() })
                .await;

            match resp {
                Err(e) => {
                    eprintln!("Error: {}; Retrying: {:?}", e, e.url());
                    continue;
                }
                Ok(resp) => return Ok(resp),
            }
        }
        panic!()
    }
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

pub struct StdOutPipeline;

impl StdOutPipeline {
    pub async fn handle_item<T>(&self, thing: Result<T, EngineError>)
    where
        T: std::fmt::Debug,
    {
        dbg!(thing.unwrap());
    }

    pub async fn pipe_out<T>(&self, things: impl Stream<Item = Result<T, EngineError>>)
    where
        T: std::fmt::Debug,
    {
        things
            .for_each(|resp| async move {
                self.handle_item(resp).await;
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::Selector;
    use futures::stream::{self, StreamExt, TryStreamExt};
    use tokio;

    #[tokio::test]
    async fn do_post() {
        /// The Url which has to be hit to set the cookies for subsequent search queries
        const COOKIE_LANDING: &str = "http://dipbt.bundestag.de/dip21.web/bt";

        let search_url =
            "http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do";

        // Start up the engine
        let client = default_client().unwrap();
        let item_pipeline = StdOutPipeline;
        // Get cookies
        client.get(COOKIE_LANDING).send_retry(3).await.unwrap();
        let resp = client.get(search_url).send_retry(3).await.unwrap();
        let sel = Selector::from_response(resp).await.unwrap();

        let mut form = sel.form_data("//form").unwrap();
        // Add the parameters needed to kick of the search
        form.push(("method".to_string(), "Suchen".to_string()));
        loop {
            let resp = client
                .post(search_url)
                .form(&form)
                .send_retry(3)
                .await
                .unwrap();
            let sel = Selector::from_response(resp).await.unwrap();
            let links = sel
                .select_links("//div[@class='tabelleGross']//a[@class='linkIntern']/@href")
                .unwrap();
            let links = stream::iter(links)
                .then(|l| client.get(dbg!(l)).send_retry(3))
                .and_then(|resp| client.get(resp.url().to_owned()).send_retry(3))
                .and_then(|resp| resp.text().map_err(Into::into));
            item_pipeline.pipe_out(links).await;
            // Prepare the form for the next iteration
            form = sel.form_data("//form").unwrap();
            // Add the parameters needed to kick of the search
            form.push(("method".to_string(), ">".to_string()));
        }
    }
}
