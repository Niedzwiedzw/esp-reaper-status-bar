#![cfg_attr(not(feature = "std"), no_std)]

use embedded_wrap_err::{IntoWrapErrExt as _, Result, WrapErrorExt as _};
use enumflags2::BitFlags;
use heapless::Vec;
use tap::prelude::*;

impl<const MAX_TRACK_COUNT: usize> ReaperStatus<MAX_TRACK_COUNT> {
    pub fn parse(response: &str) -> Result<Self> {
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
                        .and_then(|play_state| play_state
                                .parse::<u8>()
                                .into_wrap_err("parsing play_state value")
                                .and_then(|repr| {
                                    PlayState::from_repr(repr)
                                        .into_wrap_err("bad playstate")
                                }))
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
                                _ => Err("come on... forgot to change the input size?"),
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

#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub enum PlayState {
    #[default]
    Stopped = 0,
    Playing = 1,
    Paused = 3,
    Recording = 5,
    RecordPaused = 6,
}
impl PlayState {
    pub fn from_repr(repr: u8) -> Result<Self> {
        match repr {
            i if i == Self::Stopped as u8 => Ok(Self::Stopped),
            i if i == Self::Playing as u8 => Ok(Self::Playing),
            i if i == Self::Paused as u8 => Ok(Self::Paused),
            i if i == Self::Recording as u8 => Ok(Self::Recording),
            i if i == Self::RecordPaused as u8 => Ok(Self::RecordPaused),
            _unknown => Err("unknown variant"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ReaperStatus<const MAX_TRACK_COUNT: usize> {
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
