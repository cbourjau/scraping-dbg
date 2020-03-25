use futures::stream::{Stream, StreamExt};

pub struct StdOutPipeline;

impl StdOutPipeline {
    pub async fn handle_item<T>(&self, thing: T)
    where
        T: std::fmt::Debug,
    {
        dbg!(thing);
    }

    pub async fn pipe_out<T>(&self, things: impl Stream<Item = T>)
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
