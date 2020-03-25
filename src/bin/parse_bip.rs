use futures::stream::{self, StreamExt, TryStreamExt};
use futures::future::poll_fn;
use reqwest::RequestBuilder;
use tokio;
use tower::{self, Service, ServiceBuilder};

use bip_api::parsing::BipData;
use bip_api::scrapers::{
    default_client, pipelines::StdOutPipeline, retry::RetryLimit, selector::Selector, utils::open_in_browser,
};

#[tokio::main]
async fn main() {
    /// The Url which has to be hit to set the cookies for subsequent search queries
    const COOKIE_LANDING: &str = "http://dipbt.bundestag.de/dip21.web/bt";

    let search_url = "http://dipbt.bundestag.de/dip21.web/searchProcedures/advanced_search_list.do";

    let client = default_client().unwrap();
    let svc = tower::service_fn(|req_builder: RequestBuilder| req_builder.send());

    let mut svc = ServiceBuilder::new()
        .rate_limit(10, std::time::Duration::from_secs(1))
	.retry(RetryLimit::new(3))
        .service(svc);
    poll_fn(|mut cx| svc.poll_ready(&mut cx)).await.unwrap();

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
            .map_err(|e| Into::into(e))
            .and_then(|resp| async {
                let sel = Selector::from_response(resp).await?;
		// open_in_browser(&sel.body()).unwrap();
                BipData::builder()
                    .content(
                        sel.select_text("//fieldset[h1[contains(text(), 'Inhalt')]]")?.pop().clone(),
                    )
                    .summary(
                        sel.select_text("//fieldset[h1[contains(text(), 'Basisinformationen')]]")?
                            [0]
                        .clone(),
                    )
                    .tag_words(
                        sel.select_text("//fieldset[h1[contains(text(), 'Schlagwörter')]]")?.pop()
                            .clone(),
                    )
                    .build()
            });
        item_pipeline.pipe_out(links).await;
        // Prepare the form for the next iteration
        form = sel.form_data("//form").unwrap();
        // Add the parameters needed to kick of the search
        form.push(("method".to_string(), ">".to_string()));
    }
}
