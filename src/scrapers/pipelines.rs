use futures::stream::{Stream, StreamExt};

use crate::scrapers::EngineError;

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
