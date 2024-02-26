#![no_std]
use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    Drawable,
};
use embedded_wrap_err::{IntoWrapErrExt, Result, WrapErrorExt};
use reaper::{PlayState, ReaperStatus, TrackData};
use tap::prelude::*;

type ColorType = Rgb565;

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

            let track_level_color = ColorType::GREEN;
            let track_peak_color = ColorType::YELLOW;
            // pub fn draw_state(&mut self, ReaperStatus { play_state, tracks }: ReaperStatus) -> Result<()> {
            // println!("drawing state");
            const TOTAL_HEIGHT: u32 = 64;
            const TOTAL_WIDTH: u32 = 64;
            const STATUS_BAR_HEIGHT: u32 = 2;
            const MAX_TRACK_HEIGHT: u32 = TOTAL_HEIGHT - STATUS_BAR_HEIGHT;
            // status bar
            Rectangle::new(
                Point::new(0, 0),
                Size::new(TOTAL_WIDTH - 1, STATUS_BAR_HEIGHT),
            )
            .into_styled(status_color.pipe(PrimitiveStyle::with_fill))
            .draw(display)
            .into_wrap_err("drawing status bar")?;

            tracks
                .iter()
                .enumerate()
                .try_for_each(
                    |(
                        index,
                        TrackData {
                            flags,
                            last_meter_peak,
                            last_meter_pos,
                        },
                    )| {
                        Ok(())
                            .and_then(|_| {
                                let height =
                                    ((1500 + *last_meter_pos) / 1500) * (MAX_TRACK_HEIGHT as i16);
                                Rectangle::new(
                                    Point::new(index as _, (TOTAL_HEIGHT - 1) as _),
                                    Size::new(1, height as _),
                                )
                                .into_styled(track_level_color.pipe(PrimitiveStyle::with_fill))
                                .draw(display)
                                .into_wrap_err("drawing track")
                            })
                            .and_then(|_| {
                                let height =
                                    ((1500 + *last_meter_peak) / 1500) * (MAX_TRACK_HEIGHT as i16);
                                Rectangle::new(
                                    Point::new(index as _, (TOTAL_HEIGHT - 1) as _),
                                    Size::new(1, height as _),
                                )
                                .into_styled(track_peak_color.pipe(PrimitiveStyle::with_fill))
                                .draw(display)
                                .map_err(|_| "rendering failed")
                                .into_wrap_err("drawing track")
                            })
                    },
                )
                .wrap_err("drawing all tracks")?;

            Ok(())
        })
    }
}

// fn graphics_demo(delay: &mut DelayUs<u8>, display: &mut MyMatrixDisplay) -> Result<()> {
//     loop {
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
