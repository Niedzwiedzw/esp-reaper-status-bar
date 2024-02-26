use super::*;
use hal::{
    gpio::{GpioPin, Output, PushPull},
    IO,
};
use reaper::ReaperStatus;
use renderer::ReaperStatusRenderExt;

type WiringPin<const INDEX: u8> = GpioPin<Output<PushPull>, INDEX>;

#[allow(dead_code)]
mod tests {
    use embedded_hal::digital::v2::OutputPin;
    #[allow(clippy::extra_unused_type_parameters)]
    fn assert_output_pin<Pin: OutputPin>() {
        todo!()
    }
    fn assert_hub75_outputs<Outputs: hub75::Outputs>() {}

    #[test]
    fn assert_output_pins() {
        assert_output_pin::<WiringPin<34>>(); // r1,
        assert_output_pin::<WiringPin<35>>(); // g1,
        assert_output_pin::<WiringPin<32>>(); // b1,
                                              // ------------- GND
        assert_output_pin::<WiringPin<33>>(); // r2,
        assert_output_pin::<WiringPin<25>>(); // g2,
        assert_output_pin::<WiringPin<26>>(); // b2,
        assert_output_pin::<WiringPin<14>>(); // a,
        assert_output_pin::<WiringPin<12>>(); // b,
        assert_output_pin::<WiringPin<13>>(); // c,
                                              // ------------- GND
        assert_output_pin::<WiringPin<23>>(); // d,
        assert_output_pin::<WiringPin<27>>(); // e,
        assert_output_pin::<WiringPin<22>>(); // clk,
        assert_output_pin::<WiringPin<21>>(); // lat,
        assert_output_pin::<WiringPin<19>>(); // oe
        assert_hub75_outputs::<MyConnectionPins>()
    }
}
type MyConnectionPins = (
    WiringPin<25>, // R1
    WiringPin<26>, // G1
    WiringPin<27>, // BL1
    WiringPin<14>, // R2
    WiringPin<12>, // G2
    WiringPin<13>, // BL2
    WiringPin<23>, // CH_A
    WiringPin<19>, // CH_B
    WiringPin<5>,  // CH_C
    WiringPin<17>, // CH_D
    WiringPin<32>, // CH_E
    WiringPin<16>, // CLK
    WiringPin<4>,  // LAT
    WiringPin<15>, // OE
);

// type MyConnectionPins = (
//     WiringPin<18>, // r1,
//     WiringPin<5>,  // g1,
//     WiringPin<32>, // b1,
//     // ------------- GND
//     WiringPin<33>, // r2,
//     WiringPin<25>, // g2,
//     WiringPin<26>, // b2,
//     WiringPin<14>, // a,
//     WiringPin<12>, // b,
//     WiringPin<13>, // c,
//     // ------------- GND
//     WiringPin<23>, // d,
//     WiringPin<27>, // e,
//     WiringPin<22>, // clk,
//     WiringPin<21>, // lat,
//     WiringPin<19>, // oe
// );
//
pub struct MyMatrixDisplay(hub75::Hub75<MyConnectionPins>);
fn my_connection_pins(io: IO) -> MyConnectionPins {
    (
        io.pins.gpio25.into_push_pull_output(), // r1,
        io.pins.gpio26.into_push_pull_output(), // g1,
        io.pins.gpio27.into_push_pull_output(), // b1,
        // ------------- GND
        io.pins.gpio14.into_push_pull_output(), // r2,
        io.pins.gpio12.into_push_pull_output(), // g2,
        io.pins.gpio13.into_push_pull_output(), // b2,
        io.pins.gpio23.into_push_pull_output(), // a,
        io.pins.gpio19.into_push_pull_output(), // b,
        io.pins.gpio5.into_push_pull_output(),  // c,
        // ------------- GND
        io.pins.gpio17.into_push_pull_output(), // d,
        io.pins.gpio32.into_push_pull_output(), // e,
        io.pins.gpio16.into_push_pull_output(), // clk,
        io.pins.gpio4.into_push_pull_output(),  // lat,
        io.pins.gpio15.into_push_pull_output(), // oe
    )
}

impl MyMatrixDisplay {
    pub fn new(io: IO) -> Result<Self> {
        let pins = my_connection_pins(io);
        let display = hub75::Hub75::<_>::new(pins, 8);

        Ok(Self(display))
    }

    pub fn draw_state(
        &mut self,
        delay: &mut Delay,
        status: &ReaperStatus<MAX_TRACK_COUNT>,
    ) -> Result<()> {
        status
            .render(&mut self.0)
            .and_then(|_| self.0.output(delay).into_wrap_err("sending output failed"))
    }
}
