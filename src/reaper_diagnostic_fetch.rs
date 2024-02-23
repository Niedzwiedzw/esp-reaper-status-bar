use super::Result;
use crate::{Error, IntoWrapErrExt, WrapErrorExt};
use core::{fmt::Write, str::FromStr};
use embedded_nal_async::{Dns, TcpConnect};
use enumflags2::BitFlags;
use esp_println::println;
use heapless::{String, Vec};
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

impl<'stack, 'client: 'stack, T> ReaperClient<'stack, T>
where
    T: TcpConnect + 'stack,
{
    pub async fn new<D>(
        client: &'client mut HttpClient<'stack, T, D>,
        base_url: &'stack str,
    ) -> Result<Self>
    where
        D: Dns,
    {
        client
            .resource(base_url)
            .await
            .into_wrap_err("creating resource")
            .map(|http_resource| Self { http_resource })
    }

    pub async fn get_status(&mut self) -> Result<ReaperStatus> {
        let mut url = String::<256>::new();
        write!(
            &mut url,
            "/_/TRANSPORT;TRACK/0-{max_track}",
            max_track = MAX_TRACK_COUNT
        )
        .into_wrap_err("building url string")?;
        println!("fetching status from [{url}]");
        let response = {
            const MAX_RESPONSE_SIZE: usize = (MAX_TRACK_COUNT + 1) * 128;
            let mut buffer = [0; MAX_RESPONSE_SIZE];
            println!("buffer initialized");
            self.http_resource
                .request(Method::GET, &url)
                .headers(&[("Connection", "keep-alive")])
                .send(&mut buffer)
                .tap(|_| println!("request sent..."))
                .await
                .tap(|_| println!("request sent OK"))
                .into_wrap_err("sending")?
                .body()
                .read_to_end()
                .tap(|_| println!("reading body..."))
                .await
                .tap(|_| println!("reading body OK"))
                .map_err(|reading| {
                    println!("{reading:?}");
                    "reading"
                })
                .and_then(|v| {
                    Vec::<_, MAX_RESPONSE_SIZE>::new().pipe(|mut vec| {
                        vec.extend_from_slice(v)
                            .map(|_| vec)
                            .map_err(|_| "response won't fit")
                    })
                })
                .and_then(|out| String::from_utf8(out).map_err(|_| "invalid utf8"))
                .into_wrap_err("reading response")?
        };
        println!("got response: [{} bytes]", response.bytes().len());
        response.trim().lines().pipe(|mut lines| {
            lines
                .next()
                .ok_or("empty response")
                .into_wrap_err("empty response")
                .and_then(|transport| {
                    transport
                        .split_once('\t')
                        .map(|(a, b)| (a.trim(), b.trim()))
                        .ok_or("expected at least one tab")
                        .and_then(|(marker, rest)| marker.eq("TRANSPORT").then_some(rest).ok_or("expected TRANSPORT"))
                        .and_then(|transport| transport.split('\t').next().ok_or("empty transport?"))
                        .into_wrap_err("parsing TRANSPORT")
                        .map(|play_state| play_state.trim())
                        .and_then(|play_state| {
                            play_state
                                .parse::<u8>()
                                .into_wrap_err("parsing play_state value: ")
                                .wrap_err(play_state)
                                .and_then(|repr| {
                                    PlayState::from_repr(repr)
                                        .ok_or("invalid repr for playstate")
                                        .into_wrap_err("parsing playstate")
                                })
                        })
                        .and_then(|play_state| {
                            lines.map(|line| -> Result<TrackData> {
                            match line
                                .split('\t')
                                .collect::<Vec<&str, 64>>()
                                .as_slice()
                            {
                                [
                                _track,
                                 _tracknumber ,
                                 _trackname ,
                                trackflags ,
                                _volume ,
                                _pan ,
                                last_meter_peak ,
                                last_meter_pos ,
                                _width_pan2 ,
                                _panmode ,
                                _sendcnt ,
                                _recvcnt ,
                                _hwoutcnt ,
                                _color,
                            ] => {
                                    let flags = trackflags.trim()
                                        .parse::<u16>()
                                        .into_wrap_err("invalid trackflags number")
                                        .and_then(|repr| BitFlags::<TrackFlags, u16>::try_from(repr).into_wrap_err("bad TrackFlags"))
                                        .wrap_err("reading flags")?;
                                    let last_meter_peak = last_meter_peak.trim().parse::<i16>().into_wrap_err("invalid last_meter_peak")?;
                                    let last_meter_pos = last_meter_pos.trim().parse::<i16>().into_wrap_err("invalid last_meter_pos")?;
                                    Ok(TrackData {
                                        flags,
                                        last_meter_peak,
                                        last_meter_pos,
                                    })
                                }
                                _ => Err(Error::from_str("come on... forgot to change the input size?").expect("Bad string for error")),
                            }
                        })
                        .take(MAX_TRACK_COUNT)
                        .collect::<Result<Vec<_, MAX_TRACK_COUNT>>>()
                        .wrap_err("extracting track data")
                        .map(|tracks| ReaperStatus {
                                play_state,
                                tracks
                        })
                        })
                })
        })
    }
}

#[derive(Debug)]
pub struct TrackData {
    pub flags: BitFlags<TrackFlags>,
    pub last_meter_peak: i16,
    pub last_meter_pos: i16,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PlayState {
    Stopped = 0,
    Playing = 1,
    Paused = 3,
    Recording = 5,
    RecordPaused = 6,
}
impl PlayState {
    pub fn from_repr(repr: u8) -> Option<Self> {
        match repr {
            i if i == Self::Stopped as u8 => Some(Self::Stopped),
            i if i == Self::Playing as u8 => Some(Self::Stopped),
            i if i == Self::Paused as u8 => Some(Self::Stopped),
            i if i == Self::Recording as u8 => Some(Self::Stopped),
            i if i == Self::RecordPaused as u8 => Some(Self::Stopped),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ReaperStatus {
    pub play_state: PlayState,
    pub tracks: heapless::Vec<TrackData, MAX_TRACK_COUNT>,
}

#[enumflags2::bitflags]
#[repr(u16)]
#[derive(Clone, Copy, Debug)]
pub enum TrackFlags {
    Folder = 1,
    Selected = 2,
    HasFx = 4,
    Muted = 8,
    Soloed = 16,
    SoloInPlace = 32,
    RecordArmed = 64,
    RecordMonitoringOn = 128,
    RecordMonitoringAuto = 256,
    Unknown1 = 512,
}

pub const MAX_TRACK_COUNT: usize = 64;
