use super::*;
use embedded_hal::blocking::delay::DelayUs;
use embedded_wrap_err::WrapErrorExt;
use renderer::ReaperStatusRenderExt;

// type WiringPin<const INDEX: u8> = GpioPin<Output<PushPull>, INDEX>;

type MyConnectionPins = (
    embassy_rp::peripherals::PIN_2, // r1,
    embassy_rp::peripherals::PIN_3, // g1,
    embassy_rp::peripherals::PIN_4, // b1,
    // -----------------------------// GND
    embassy_rp::peripherals::PIN_5,  // r2,
    embassy_rp::peripherals::PIN_8,  // g2,
    embassy_rp::peripherals::PIN_9,  // b2,
    embassy_rp::peripherals::PIN_10, // a,
    embassy_rp::peripherals::PIN_16, // b,
    embassy_rp::peripherals::PIN_18, // c,
    // ------------------------------ GND
    embassy_rp::peripherals::PIN_20, // d,
    embassy_rp::peripherals::PIN_22, // e,
    embassy_rp::peripherals::PIN_11, // clk,
    embassy_rp::peripherals::PIN_12, // lat,
    embassy_rp::peripherals::PIN_13, // oe
);

type MyOutputConnectionPins = (
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
    Output<'static>,
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
pub struct MyMatrixDisplay(hub75::Hub75<MyOutputConnectionPins>);
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

fn to_output(pins: MyConnectionPins) -> MyOutputConnectionPins {
    macro_rules! output {
        ($pin:expr) => {
            Output::new($pin, Level::Low)
        };
    }
    (
        output!(pins.0),
        output!(pins.1),
        output!(pins.2),
        output!(pins.3),
        output!(pins.4),
        output!(pins.5),
        output!(pins.6),
        output!(pins.7),
        output!(pins.8),
        output!(pins.9),
        output!(pins.10),
        output!(pins.11),
        output!(pins.12),
        output!(pins.13),
    )
}

impl MyMatrixDisplay {
    pub fn new(pins: MyConnectionPins) -> Result<Self> {
        let display = hub75::Hub75::<_>::new(pins.pipe(to_output), 1);

        Ok(Self(display))
    }
    pub fn draw(&mut self, delay: &mut impl DelayUs<u8>) -> Result<()> {
        self.0.output(delay).into_wrap_err("displaying output")
    }

    pub fn update_display_data(&mut self, status: &ReaperStatus<MAX_TRACK_COUNT>) -> Result<()> {
        status.render(&mut self.0).wrap_err("updating render data")
    }
}
