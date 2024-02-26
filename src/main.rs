#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Config, Stack, StackResources,
};
use embassy_time::{
    with_timeout,
    // with_timeout,
    Duration,
    Timer,
};
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use embedded_wrap_err::{IntoWrapErrExt as _, Result, WrapErrorExt as _};
use esp32_hal as hal;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    initialize,
    wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState},
    EspWifiInitFor,
};
use hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup, Delay,
    Rng, IO,
};
use static_cell::make_static;
use status_bar_display::MyMatrixDisplay;

pub mod reaper_diagnostic_fetch;
pub mod status_bar_display;

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

type NetworkStack = &'static Stack<WifiDevice<'static, WifiStaDevice>>;

struct GlobalContext {
    network_stack: NetworkStack,
    display: status_bar_display::MyMatrixDisplay,
    delay: Delay,
}

async fn setup(spawner: Spawner) -> Result<GlobalContext> {
    esp_println::logger::init_logger(log::LevelFilter::Warn);
    let peripherals = Peripherals::take();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let display =
        crate::status_bar_display::MyMatrixDisplay::new(io).wrap_err("initializing display")?;
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    // #[cfg(target_arch = "xtensa")]
    let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    // #[cfg(target_arch = "riscv32")]
    // let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .into_wrap_err("initializing wifi")?;

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice)
        .into_wrap_err("initializing wifi interface and controller")?;

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack: &'static _ = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<4>::new()),
        seed
    ));

    let delay = Delay::new(&clocks);

    spawner
        .spawn(connection(controller))
        .into_wrap_err("spawning connection controller")?;
    spawner
        .spawn(net_task(stack))
        .into_wrap_err("spawning net task")?;

    Ok(GlobalContext {
        network_stack: stack,
        display,
        delay,
    })
}

async fn actual_main(
    _spawner: Spawner,
    network_stack: NetworkStack,
    display: &mut MyMatrixDisplay,
    delay: &mut Delay,
) -> Result<()> {
    // graphics_demo(delay, display)?;

    loop {
        if network_stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = network_stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let client_state = TcpClientState::<2, IO_BUFFER_SIZE, IO_BUFFER_SIZE>::new();
    let tcp_client = TcpClient::new(network_stack, &client_state);
    println!("created a tcp client");
    let dns_socket = DnsSocket::new(network_stack);
    let mut client = reqwless::client::HttpClient::new(&tcp_client, &dns_socket);

    let mut client = reaper_diagnostic_fetch::ReaperClient::new(&mut client, ESP_REAPER_BASE_URL)
        .await
        .wrap_err("building reaper client")?;
    println!("created an http client");
    loop {
        with_timeout(Duration::from_millis(5000), client.get_status())
            .await
            .into_wrap_err("timeout occurred")
            .and_then(|out| out)
            .wrap_err("fetching reaper status")
            .and_then(|status| {
                // println!(
                //     "play state: {:?}\ttrack_count:{}",
                //     status.play_state,
                //     status.tracks.len()
                // );
                // loop {
                // display.draw_state(status).ok();
                display.draw_state(delay, &status)

                // }
            })?;
        Timer::after(Duration::from_millis(50)).await;
    }
}

macro_rules! debug_env {
    ($name:ident) => {{
        let name = stringify!($name);
        println!("{name}={value}", value = $name);
    }};
}

#[main]
async fn main(spawner: Spawner) -> ! {
    debug_env!(ESP_WIFI_SSID);
    debug_env!(ESP_WIFI_PASSWORD);
    debug_env!(ESP_REAPER_BASE_URL);
    let GlobalContext {
        network_stack,
        mut display,
        mut delay,
    } = setup(spawner).await.expect("failed to setup network stack");
    loop {
        match actual_main(spawner, network_stack, &mut display, &mut delay).await {
            Ok(_) => println!("app just finished"),
            Err(message) => {
                println!("ERROR: {message}. (restarting)");

                Timer::after(Duration::from_millis(3000)).await;
            }
        }
    }
}

#[allow(clippy::single_match)]
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                println!("disconnected, reconecting");
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: ESP_WIFI_SSID.try_into().expect("bad ssid"),
                password: ESP_WIFI_PASSWORD.try_into().expect("bad password"),
                ..Default::default()
            });
            controller
                .set_configuration(&client_config)
                .expect("setting configuration");
            println!("Starting wifi");
            controller.start().await.expect("starting controller");
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: NetworkStack) {
    stack.run().await
}
