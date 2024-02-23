use self::reaper_diagnostic_fetch::ReaperStatus;

use super::*;
use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    primitives::{Primitive as _, PrimitiveStyle, Rectangle},
    Drawable as _,
};
use hal::{
    gpio::{GpioPin, Output, PushPull},
    IO,
};

type WiringPin<const INDEX: u8> = GpioPin<Output<PushPull>, INDEX>;

mod tests {
    use super::*;

    use embedded_hal::digital::v2::OutputPin;
    fn assert_output_pin<Pin: OutputPin>() {
        todo!()
    }
    fn assert_hub75_outputs<Outputs: hub75::Outputs>() {}

    fn assert_output_pins() {
        assert_output_pin::<WiringPin<4>>();
        assert_output_pin::<WiringPin<2>>();
        assert_output_pin::<WiringPin<32>>();
        assert_output_pin::<WiringPin<33>>();
        assert_output_pin::<WiringPin<25>>();
        assert_output_pin::<WiringPin<26>>();
        assert_output_pin::<WiringPin<27>>();
        assert_output_pin::<WiringPin<14>>();
        assert_output_pin::<WiringPin<12>>();
        assert_output_pin::<WiringPin<13>>();
        assert_output_pin::<WiringPin<5>>();
        assert_output_pin::<WiringPin<21>>();
        assert_output_pin::<WiringPin<19>>();
        assert_output_pin::<WiringPin<18>>();
        assert_hub75_outputs::<MyConnectionPins>()
    }
}

type MyConnectionPins = (
    WiringPin<4>,  // r1,
    WiringPin<2>,  // g1,
    WiringPin<32>, // b1,
    WiringPin<33>, // r2,
    WiringPin<25>, // g2,
    WiringPin<26>, // b2,
    WiringPin<27>, // a,
    WiringPin<14>, // b,
    WiringPin<12>, // c,
    WiringPin<13>, // d,
    WiringPin<5>,  // e,
    WiringPin<21>, // clk,
    WiringPin<19>, // lat,
    WiringPin<18>, // oe
);
//
pub struct MyMatrixDisplay(hub75::Hub75<MyConnectionPins>);
impl MyMatrixDisplay {
    pub fn new(peripherals: &mut Peripherals) -> Result<Self> {
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let pins: MyConnectionPins = (
            io.pins.gpio4.into_push_pull_output(),  // r1,
            io.pins.gpio2.into_push_pull_output(),  // g1,
            io.pins.gpio32.into_push_pull_output(), // b1,
            io.pins.gpio33.into_push_pull_output(), // r2,
            io.pins.gpio25.into_push_pull_output(), // g2,
            io.pins.gpio26.into_push_pull_output(), // b2,
            io.pins.gpio27.into_push_pull_output(), // a,
            io.pins.gpio14.into_push_pull_output(), // b,
            io.pins.gpio12.into_push_pull_output(), // c,
            io.pins.gpio13.into_push_pull_output(), // d,
            io.pins.gpio5.into_push_pull_output(),  // e,
            io.pins.gpio21.into_push_pull_output(), // clk,
            io.pins.gpio19.into_push_pull_output(), // lat,
            io.pins.gpio18.into_push_pull_output(), // oe
        );
        let mut display = hub75::Hub75::<_>::new(pins, 4);

        Ok(Self(display))
    }

    pub fn draw_state(&self, ReaperStatus { play_state, tracks }: ReaperStatus) -> Result<()> {
        let fill = PrimitiveStyle::with_fill(Rgb565::new(30, 80, 120));

        Rectangle::new(Point::new(5, 5), Size::new(16, 16))
            .into_styled(fill)
            .draw(&mut self.0)
            .into_wrap_err("drawing state")
    }
}
