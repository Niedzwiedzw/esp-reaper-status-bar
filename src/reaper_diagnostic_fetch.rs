use crate::{MAX_RESPONSE_SIZE, MAX_TRACK_COUNT};
use core::fmt::Write;
use embedded_nal_async::{Dns, TcpConnect};
use embedded_wrap_err::{IntoWrapErrDebugExt, IntoWrapErrExt, Result};
use heapless::{String, Vec};
use reaper::ReaperStatus;
use reqwless::{
    client::{HttpClient, HttpResource},
    request::{Method, RequestBuilder},
};
use tap::prelude::*;

pub struct ReaperClient<'stack, T>
where
    T: TcpConnect + 'stack,
{
    http_resource: HttpResource<'stack, T::Connection<'stack>>,
}

fn zero(buffer: &mut [u8]) {
    buffer.iter_mut().for_each(|v| *v = 0)
}

impl<'stack, 'client: 'stack, T> ReaperClient<'stack, T>
where
    T: TcpConnect + 'stack,
{
    pub async fn new<D>(client: &'client mut HttpClient<'stack, T, D>, base_url: &'stack str) -> Result<Self>
    where
        D: Dns,
    {
        client
            .resource(base_url)
            .await
            .into_wrap_err_dbg("creating resource")
            .map(|http_resource| Self { http_resource })
    }

    pub async fn get_status(&mut self) -> Result<ReaperStatus<MAX_TRACK_COUNT>> {
        let mut url = String::<256>::new();
        write!(&mut url, "/_/TRANSPORT;TRACK/0-{max_track}", max_track = MAX_TRACK_COUNT).into_wrap_err_dbg("building url string")?;
        // println!("fetching status from [{url}]");
        {
            let mut buffer = [0; MAX_RESPONSE_SIZE];

            // println!("buffer initialized");
            self.http_resource
                .request(Method::GET, &url)
                .headers(&[("Connection", "keep-alive")])
                .send(&mut buffer)
                // .tap(|_| println!("request sent..."))
                .await
                // .tap(|_| println!("request sent OK"))
                .into_wrap_err_dbg("sending")?
                .body()
                .read_to_end()
                // .tap(|_| println!("reading body..."))
                .await
                // .tap(|_| println!("reading body OK"))
                .into_wrap_err_dbg("reading")
                .and_then(|v| {
                    Vec::<_, MAX_RESPONSE_SIZE>::new().pipe(|mut vec| {
                        vec.extend_from_slice(v)
                            .map(|_| vec)
                            .map_err(|_| "response won't fit")
                    })
                })
                .and_then(|out| String::from_utf8(out).into_wrap_err_dbg("invalid utf8"))
                .into_wrap_err("reading response")?
        }
        .pipe_ref(|v| v.as_str().pipe(ReaperStatus::parse))
    }
}
