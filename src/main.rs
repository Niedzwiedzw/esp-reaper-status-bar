#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Config, Stack, StackResources,
};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::channel::{Channel, Sender};
use embassy_sync::{
    blocking_mutex::{raw::CriticalSectionRawMutex, CriticalSectionMutex},
    channel::Receiver,
};
use embassy_time::{with_timeout, Duration, Ticker, Timer};
use embedded_svc::{
    channel,
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use embedded_wrap_err::{IntoWrapErrExt, Result, WrapErrorExt as _};
use log::error;
use portable_atomic::{AtomicUsize, Ordering};
use reaper::ReaperStatus;
use renderer::ReaperStatusRenderExt;
use static_cell::StaticCell;
// use status_bar_display::MyMatrixDisplay;
use tap::Pipe;
use {defmt_rtt as _, panic_probe as _};

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_rp::bind_interrupts;
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

pub mod reaper_diagnostic_fetch;
// pub mod status_bar_display;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// const RX_BUFFER_SIZE: usize = BUFFER_SIZE;
// const TX_BUFFER_SIZE: usize = BUFFER_SIZE;

pub const MAX_TRACK_COUNT: usize = 64;

const MAX_HEADER_SIZE: usize = 512;
const MAX_TRACK_LINE_SIZE: usize = 128;

pub const ESP_WIFI_SSID: &str = env!("ESP_WIFI_SSID");
pub const ESP_WIFI_PASSWORD: &str = env!("ESP_WIFI_PASSWORD");
pub const ESP_REAPER_BASE_URL: &str = env!("ESP_REAPER_BASE_URL");
pub const MAX_RESPONSE_SIZE: usize = (MAX_TRACK_COUNT + 1) * MAX_TRACK_LINE_SIZE;
pub const IO_BUFFER_SIZE: usize = MAX_HEADER_SIZE + MAX_RESPONSE_SIZE;
pub const MAX_ERROR_SIZE: usize = 128;

const MAX_MESSAGE_SIZE: usize = 8;

static REAPER_STATE_CHANNEL: Channel<CriticalSectionRawMutex, ReaperStatus<MAX_TRACK_COUNT>, MAX_MESSAGE_SIZE> = Channel::new();

#[embassy_executor::task]
async fn wifi_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

type NetworkStack = &'static Stack<cyw43::NetDriver<'static>>;

// struct GlobalContext {
//     network_stack: NetworkStack,
// }

// #[embassy_executor::task]
// async fn keep_redrawing_screen(
//     updates: Receiver<
//         'static,
//         CriticalSectionRawMutex,
//         ReaperStatus<MAX_TRACK_COUNT>,
//         MAX_MESSAGE_SIZE,
//     >,
//     // mut display: MyMatrixDisplay,
// ) {
//     let mut buffer = ReaperStatus::default();

//     let mut ticker = Ticker::every(Duration::from_millis(50));

//     loop {
//         ticker.next().await;
//         if let Ok(update) = updates.try_receive() {
//             buffer = update;
//         }

//         if let Err(message) = display.draw_state(&buffer) {
//             error!("{message}");
//         }
//     }
// }
const FIRMWARE_FW: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
const FIRMWARE_CLM: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

const STACK_RESOURCES_COUNT: usize = 3;

static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<STACK_RESOURCES_COUNT>> = StaticCell::new();

async fn setup(spawner: Spawner) -> Result<NetworkStack> {
    info!("setup");
    let peripherals = embassy_rp::init(Default::default());
    let pwr = Output::new(peripherals.PIN_23, Level::Low);
    let cs = Output::new(peripherals.PIN_25, Level::High);
    let mut pio = Pio::new(peripherals.PIO0, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, peripherals.PIN_24, peripherals.PIN_29, peripherals.DMA_CH0);
    // let display =
    //     crate::status_bar_display::MyMatrixDisplay::new(io).wrap_err("
    // initializing display")?;
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE_FW).await;

    unwrap!(spawner.spawn(wifi_task(runner)));
    control.init(FIRMWARE_CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    let config = Config::dhcpv4(Default::default());
    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    let stack = &*STACK.init(Stack::new(net_device, config, RESOURCES.init(StackResources::<STACK_RESOURCES_COUNT>::new()), seed));
    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        //control.join_open(WIFI_NETWORK).await;
        match control.join_wpa2(ESP_WIFI_SSID, ESP_WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    // let system = peripherals.SYSTEM.split();

    // let clocks = ClockControl::max(system.clock_control).freeze();
    // // #[cfg(target_arch = "xtensa")]
    // let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    // // #[cfg(target_arch = "riscv32")]
    // // let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    // let init = initialize(
    //     EspWifiInitFor::Wifi,
    //     timer,
    //     Rng::new(peripherals.RNG),
    //     system.radio_clock_control,
    //     &clocks,
    // )
    // .into_wrap_err("initializing wifi")?;

    // let wifi = peripherals.WIFI;
    // let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(&init, wifi,
    // WifiStaDevice)     .into_wrap_err("initializing wifi interface and
    // controller")?;

    // let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    // embassy::init(&clocks, timer_group0);

    // let config = Config::dhcpv4(Default::default());

    // let seed = 1234; // very random, very secure seed

    // // Init network stack
    // let stack: &'static _ = &*make_static!(Stack::new(
    //     wifi_interface,
    //     config,
    //     make_static!(StackResources::<4>::new()),
    //     seed
    // ));

    // spawner
    //     .spawn(connection(controller))
    //     .into_wrap_err("spawning connection controller")?;
    // spawner
    //     .spawn(net_task(stack))
    //     .into_wrap_err("spawning net task")?;

    // spawner
    //     .spawn(keep_redrawing_screen(
    //         REAPER_STATE_CHANNEL.receiver(),
    //         display,
    //     ))
    //     .into_wrap_err("spawning redrawing loop")?;

    Ok(stack)
}

async fn actual_main(_spawner: Spawner, stack: NetworkStack, sender: Sender<'static, CriticalSectionRawMutex, ReaperStatus<MAX_TRACK_COUNT>, MAX_MESSAGE_SIZE>) -> Result<()> {
    // graphics_demo(delay, display)?;

    let client_state = TcpClientState::<3, IO_BUFFER_SIZE, IO_BUFFER_SIZE>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    info!("created a tcp client");
    let dns_socket = DnsSocket::new(stack);
    info!("created a dns socket");
    let mut client = reqwless::client::HttpClient::new(&tcp_client, &dns_socket);

    info!("created a http client");
    let mut client = reaper_diagnostic_fetch::ReaperClient::new(&mut client, ESP_REAPER_BASE_URL)
        .await
        .wrap_err("building reaper client")?;
    info!("created an reaper client");

    loop {
        let status = with_timeout(Duration::from_millis(5000), client.get_status())
            .await
            .into_wrap_err("timeout occurred")
            .and_then(|out| out)
            .wrap_err("fetching reaper status")?;
        sender.send(status).await;

        Timer::after(Duration::from_millis(50)).await;
    }
}

macro_rules! debug_env {
    ($name:ident) => {{
        let name = stringify!($name);
        info!("{}={}", name, $name);
    }};
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    info!("board is booting up");
    debug_env!(ESP_WIFI_SSID);
    debug_env!(ESP_WIFI_PASSWORD);
    debug_env!(ESP_REAPER_BASE_URL);
    let stack = setup(spawner).await.expect("failed to setup network stack");
    loop {
        match actual_main(spawner, stack, REAPER_STATE_CHANNEL.sender()).await {
            Ok(_) => info!("app just finished"),
            Err(message) => {
                info!("ERROR: {}. (restarting)", message);

                Timer::after(Duration::from_millis(3000)).await;
            }
        }
    }
}

// #[allow(clippy::single_match)]
// #[embassy_executor::task]
// async fn connection(mut controller: WifiController<'static>) {
//     info!("start connection task");
//     info!("Device capabilities: {:?}", controller.get_capabilities());
//     loop {
//         match esp_wifi::wifi::get_wifi_state() {
//             WifiState::StaConnected => {
//                 // wait until we're no longer connected
//                 controller.wait_for_event(WifiEvent::StaDisconnected).await;
//                 info!("disconnected, reconecting");
//                 Timer::after(Duration::from_millis(5000)).await
//             }
//             _ => {}
//         }
//         if !matches!(controller.is_started(), Ok(true)) {
//             let client_config = Configuration::Client(ClientConfiguration {
//                 ssid: ESP_WIFI_SSID.try_into().expect("bad ssid"),
//                 password: ESP_WIFI_PASSWORD.try_into().expect("bad
// password"),                 ..Default::default()
//             });
//             controller
//                 .set_configuration(&client_config)
//                 .expect("setting configuration");
//             info!("Starting wifi");
//             controller.start().await.expect("starting controller");
//             info!("Wifi started!");
//         }
//         info!("About to connect...");

//         match controller.connect().await {
//             Ok(_) => info!("Wifi connected!"),
//             Err(e) => {
//                 info!("Failed to connect to wifi: {e:?}");
//                 Timer::after(Duration::from_millis(5000)).await
//             }
//         }
//     }
// }

// #[embassy_executor::task]
// async fn net_task(stack: NetworkStack) {
//     stack.run().await
// }
