use tower::retry::Policy;
use futures::future;
use reqwest::{RequestBuilder, Response, Error};

#[derive(Clone, Debug)]
pub struct RetryLimit {
    remaining_tries: usize,
}

impl Policy<RequestBuilder, Response, Error> for RetryLimit {
    type Future = future::Ready<Self>;

    fn retry(&self, _req: &RequestBuilder, result: Result<&Response, &Error>) -> Option<Self::Future> {
        match result {
            Ok(resp) => {
		match resp.error_for_status_ref() {
		    Ok(_resp) => None,
		    Err(_e) => self.should_retry()
		}
            },
            Err(_) => {
                // We should probably just give up but lets keep trying even in this case.
		self.should_retry()
            }
        }
    }

    fn clone_request(&self, req: &RequestBuilder) -> Option<RequestBuilder> {
        req.try_clone()
    }
}

impl RetryLimit {
    pub fn new(remaining_tries: usize) -> Self {
	Self { remaining_tries }
    }
    
    fn should_retry(&self) -> Option<future::Ready<Self>> {
	let remaining_tries = self.remaining_tries - 1;
	if self.remaining_tries > 0 {
            Some(future::ready(RetryLimit{ remaining_tries }))
        } else {
            None
        }
    }
}
