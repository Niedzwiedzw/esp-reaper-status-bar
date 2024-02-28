#![cfg_attr(not(feature = "std"), no_std)]

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{Rgb565, RgbColor, WebColors},
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    Drawable,
};
use embedded_wrap_err::{IntoWrapErrDebugExt, IntoWrapErrExt, Result, WrapErrorExt};
use reaper::{PlayState, ReaperStatus, TrackData, TrackFlags};
use tap::prelude::*;

type ColorType = Rgb565;

pub fn minus_decibel_to_height(minus_decibel: i16) -> f32 {
    1. + (minus_decibel.min(0) as f32 / 1500.)
}

#[extension_traits::extension(pub trait ReaperStatusRenderExt)]
impl<const MAX_TRACK_COUNT: usize> ReaperStatus<MAX_TRACK_COUNT> {
    fn render<E, D>(&self, display: &mut D) -> Result<()>
    where
        E: core::fmt::Debug,
        D: embedded_graphics::draw_target::DrawTarget<Color = ColorType, Error = E>,
    {
        self.pipe(|Self { play_state, tracks }| -> Result<()> {
            let status_color = match play_state {
                PlayState::Stopped => ColorType::BLUE,
                PlayState::Playing => ColorType::GREEN,
                PlayState::Paused => ColorType::BLUE,
                PlayState::Recording => ColorType::RED,
                PlayState::RecordPaused => ColorType::BLUE,
            };

            const TRACK_LEVEL_COLOR: ColorType = ColorType::GREEN;
            const TRACK_PEAK_COLOR: ColorType = ColorType::YELLOW;
            const TRACK_PEAK_ERROR: ColorType = ColorType::RED;
            const TRACK_MUTED_COLOR: ColorType = ColorType::CSS_DIM_GRAY;
            // pub fn draw_state(&mut self, ReaperStatus { play_state, tracks }:
            // ReaperStatus) -> Result<()> { println!("drawing state");
            const TOTAL_HEIGHT: u32 = 64;
            const TOTAL_WIDTH: u32 = 64;
            const STATUS_BAR_HEIGHT: u32 = 2;
            const MAX_TRACK_HEIGHT: u32 = TOTAL_HEIGHT - STATUS_BAR_HEIGHT;
            // status bar
            Rectangle::new(Point::new(0, 0), Size::new(TOTAL_WIDTH, STATUS_BAR_HEIGHT))
                .into_styled(status_color.pipe(PrimitiveStyle::with_fill))
                .draw(display)
                .into_wrap_err_dbg("drawing status bar")?;

            tracks
                .iter()
                .enumerate()
                .try_for_each(|(index, TrackData { flags, last_meter_peak, last_meter_pos })| {
                    let height = |decibel_value: i16| {
                        ((minus_decibel_to_height(decibel_value) * MAX_TRACK_HEIGHT as f32) as u32).tap_dbg(|_height| {
                            #[cfg(feature = "std")]
                            println!("decibel_value: {decibel_value}; height: {_height}");
                        })
                    };
                    let position = |index: usize, decibel_value| Point::new(index as _, (STATUS_BAR_HEIGHT + MAX_TRACK_HEIGHT - height(decibel_value)) as _);
                    let rectangle = |index, decibel_value| Rectangle::new(position(index, decibel_value), Size::new(1, height(decibel_value)));
                    let max_value = last_meter_pos.max(last_meter_peak);
                    Ok(())
                        .and_then(|_| {
                            rectangle(index as _, *last_meter_pos)
                                .into_styled(TRACK_LEVEL_COLOR.pipe(PrimitiveStyle::with_fill))
                                .draw(display)
                                .into_wrap_err_dbg("drawing track")
                        })
                        .and_then(|_| {
                            rectangle(index as _, *last_meter_peak)
                                .into_styled(TRACK_PEAK_COLOR.pipe(PrimitiveStyle::with_fill))
                                .draw(display)
                                .into_wrap_err_dbg("drawing track")
                        })
                        .and_then(|_| {
                            max_value
                                .ge(&0)
                                .then(|| {
                                    rectangle(index as _, *max_value)
                                        .into_styled(TRACK_PEAK_ERROR.pipe(PrimitiveStyle::with_fill))
                                        .draw(display)
                                        .into_wrap_err_dbg("drawing track")
                                })
                                .unwrap_or(Ok(()))
                        })
                        .and_then(|_| {
                            flags
                                .contains(TrackFlags::Muted)
                                .then(|| {
                                    rectangle(index as _, *max_value)
                                        .into_styled(TRACK_MUTED_COLOR.pipe(PrimitiveStyle::with_fill))
                                        .draw(display)
                                        .into_wrap_err_dbg("drawing track")
                                })
                                .unwrap_or(Ok(()))
                        })
                })
                .wrap_err("drawing all tracks")?;

            Ok(())
        })
    }
}

// fn graphics_demo(delay: &mut DelayUs<u8>, display: &mut MyMatrixDisplay) ->
// Result<()> {     loop {
//         if let Err(message) = display.draw_state(
//             delay,
//             &ReaperStatus {
//                 play_state: PlayState::Recording,
//                 tracks: Vec::new().tap_mut(|v| {
//                     v.push(TrackData {
//                         flags: BitFlags::empty(),
//                         last_meter_peak: -1500,
//                         last_meter_pos: -1500,
//                     })
//                     .ok();
//                     v.push(TrackData {
//                         flags: BitFlags::empty(),
//                         last_meter_peak: -750,
//                         last_meter_pos: -750,
//                     })
//                     .ok();
//                     v.push(TrackData {
//                         flags: BitFlags::empty(),
//                         last_meter_peak: -300,
//                         last_meter_pos: -300,
//                     })
//                     .ok();
//                     v.push(TrackData {
//                         flags: BitFlags::empty(),
//                         last_meter_peak: 0,
//                         last_meter_pos: 0,
//                     })
//                     .ok();
//                 }),
//             },
//         ) {
//             println!("drawing failed: {message}");
//         }
//     }
// }
