use super::*;
use renderer::ReaperStatusRenderExt;

// type WiringPin<const INDEX: u8> = GpioPin<Output<PushPull>, INDEX>;

type MyConnectionPins = (
    Output<'static>, // R1
    Output<'static>, // G1
    Output<'static>, // BL1
    Output<'static>, // R2
    Output<'static>, // G2
    Output<'static>, // BL2
    Output<'static>, // CH_A
    Output<'static>, // CH_B
    Output<'static>, // CH_C
    Output<'static>, // CH_D
    Output<'static>, // CH_E
    Output<'static>, // CLK
    Output<'static>, // LAT
    Output<'static>, // OE
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
// fn my_connection_pins(io: &'static Peripherals) -> MyConnectionPins {
//     (
//         Output::new(&io.PIN_25, Level::Low), // r1,
//         Output::new(&io.PIN_26, Level::Low), // g1,
//         Output::new(&io.PIN_27, Level::Low), // b1,
//         // ------------- GND
//         Output::new(&io.PIN_14, Level::Low), // r2,
//         Output::new(&io.PIN_12, Level::Low), // g2,
//         Output::new(&io.PIN_13, Level::Low), // b2,
//         Output::new(&io.PIN_23, Level::Low), // a,
//         Output::new(&io.PIN_19, Level::Low), // b,
//         Output::new(&io.PIN_5, Level::Low),  // c,
//         // ------------- GND
//         Output::new(&io.PIN_17, Level::Low), // d,
//         Output::new(&io.PIN_0, Level::Low),  // e,
//         Output::new(&io.PIN_16, Level::Low), // clk,
//         Output::new(&io.PIN_4, Level::Low),  // lat,
//         Output::new(&io.PIN_15, Level::Low), // oe
//     )
// }

impl MyMatrixDisplay {
    pub fn new(pins: MyConnectionPins) -> Result<Self> {
        let display = hub75::Hub75::<_>::new(pins, 8);

        Ok(Self(display))
    }

    pub fn draw_state(&mut self, status: &ReaperStatus<MAX_TRACK_COUNT>) -> Result<()> {
        status
            .render(&mut self.0)
            .and_then(|_| self.0.output().into_wrap_err("sending output failed"))
    }
}
