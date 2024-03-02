#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Config, Stack, StackResources,
};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{
    gpio::{Level, Output},
    pac::Interrupt::CLOCKS_IRQ,
};
use embassy_sync::channel::{Channel, Sender};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Receiver};
use embassy_time::{with_timeout, Delay, Duration, Ticker, Timer};
use embedded_wrap_err::{IntoWrapErrExt, Result, WrapErrorExt as _};
use futures::FutureExt;
use log::error;
use reaper::ReaperStatus;
use static_cell::StaticCell;
use status_bar_display::MyMatrixDisplay;
use tap::prelude::*;
// use status_bar_display::MyMatrixDisplay;
use tap::Pipe;
use {defmt_rtt as _, panic_probe as _};

use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use {defmt_rtt as _, panic_probe as _};

pub mod reaper_diagnostic_fetch;
pub mod status_bar_display;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const ESP_WIFI_SSID: &str = env!("ESP_WIFI_SSID");
const ESP_WIFI_PASSWORD: &str = env!("ESP_WIFI_PASSWORD");
const ESP_REAPER_BASE_URL: &str = env!("ESP_REAPER_BASE_URL");

const MAX_HEADER_SIZE: usize = 512;
const MAX_TRACK_COUNT: usize = 64;
const MAX_TRACK_LINE_SIZE: usize = 128;

const MAX_RESPONSE_SIZE: usize = (MAX_TRACK_COUNT + 1) * MAX_TRACK_LINE_SIZE;

const IO_BUFFER_SIZE: usize = MAX_HEADER_SIZE + MAX_RESPONSE_SIZE;

const MAX_MESSAGE_COUNT: usize = 8;

static REAPER_STATE_CHANNEL: Channel<CriticalSectionRawMutex, ReaperStatus<MAX_TRACK_COUNT>, MAX_MESSAGE_COUNT> = Channel::new();

// https://github.com/embassy-rs/embassy/issues/1736
// https://github.com/probe-rs/probe-rs/pull/1603
#[cortex_m_rt::pre_init]
unsafe fn pre_init() {
    // Reset spinlock 31
    core::arch::asm!(
        "
        ldr r0, =1
        ldr r1, =0xd000017c
        str r0, [r1]
        "
    );
}

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Device is starting up");
    let peripherals = embassy_rp::init(Default::default());

    info!("peripherals OK");

    let display = {
        MyMatrixDisplay::new((
            peripherals.PIN_2,
            peripherals.PIN_3,
            peripherals.PIN_4,
            peripherals.PIN_5,
            peripherals.PIN_8,
            peripherals.PIN_9,
            peripherals.PIN_10,
            peripherals.PIN_16,
            peripherals.PIN_18,
            peripherals.PIN_20,
            peripherals.PIN_22,
            peripherals.PIN_11,
            peripherals.PIN_12,
            peripherals.PIN_13,
        ))
        .wrap_err("creating matrix display")
        .pipe(|display| unwrap!(display))
        .pipe(|display| MY_MATRIX_DISPLAY.init(display))
    };
    let wifi_setup_context = SetupWifiContext {
        pwr_pin: peripherals.PIN_23,
        cs_pin: peripherals.PIN_25,
        dio_pin: peripherals.PIN_24,
        clk_pin: peripherals.PIN_29,
        dma_ch0: peripherals.DMA_CH0,
        pio0_pin: peripherals.PIO0,
    };
    info!("WIFI pins OK");
    // CORE 1
    embassy_rp::multicore::spawn_core1(
        peripherals.CORE1,
        {
            #[allow(static_mut_refs)]
            unsafe {
                &mut CORE1_STACK
            }
        },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            info!("running core 1: display redrawing");
            executor1.run(|spawner| unwrap!(spawner.spawn(keep_redrawing_screen(REAPER_STATE_CHANNEL.receiver(), display))));
        },
    );
    // CORE 0
    {
        info!("display OK");
        info!("running core 0");
        let executor0 = EXECUTOR0.init(Executor::new());
        executor0.run(|spawner| {
            info!("spawning core 0: embassy main");
            unwrap!(spawner.spawn(embassy_main(spawner, wifi_setup_context)))
        });
    }
}

static MY_MATRIX_DISPLAY: StaticCell<MyMatrixDisplay> = StaticCell::new();

#[embassy_executor::task]
async fn keep_redrawing_screen(updates: Receiver<'static, CriticalSectionRawMutex, ReaperStatus<MAX_TRACK_COUNT>, MAX_MESSAGE_COUNT>, display: &'static mut MyMatrixDisplay) {
    info!("screen task running");
    let mut delay = Delay;
    display
        .update_display_data(&Default::default())
        .expect("could not redraw even once");
    loop {
        info!("checking if there's some data");
        if let Ok(updated) = {
            let updates = updates.try_receive();
            info!("checked");
            updates
        } {
            match display
                .update_display_data(&updated)
                .wrap_err("received data but couldn't render")
            {
                Ok(_) => {
                    info!("updated the screen");
                }
                Err(message) => {
                    debug!("reason: {}", message);
                }
            }
        }

        info!("starting to draw");
        if let Err(message) = {
            let draw = display.draw(&mut delay);
            info!("draw finished");
            draw
        } {
            error!("couldn't draw: {message}");
        }
    }
}
#[embassy_executor::task]
async fn wifi_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

type NetworkStack = &'static Stack<cyw43::NetDriver<'static>>;

const FIRMWARE_FW: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
const FIRMWARE_CLM: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

const STACK_RESOURCES_COUNT: usize = 3;

static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<STACK_RESOURCES_COUNT>> = StaticCell::new();

struct SetupWifiContext {
    pwr_pin: embassy_rp::peripherals::PIN_23,
    cs_pin: embassy_rp::peripherals::PIN_25,
    dio_pin: embassy_rp::peripherals::PIN_24,
    clk_pin: embassy_rp::peripherals::PIN_29,
    dma_ch0: embassy_rp::peripherals::DMA_CH0,
    pio0_pin: embassy_rp::peripherals::PIO0,
}

async fn setup_wifi(
    spawner: Spawner,
    SetupWifiContext {
        pwr_pin,
        cs_pin,
        dio_pin,
        clk_pin,
        dma_ch0,
        pio0_pin,
    }: SetupWifiContext,
) -> Result<NetworkStack> {
    info!("setup");

    let pwr = Output::new(pwr_pin, Level::Low);
    let cs = Output::new(cs_pin, Level::High);
    let mut pio = Pio::new(pio0_pin, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, dio_pin, clk_pin, dma_ch0);
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

    Ok(stack)
}

async fn actual_main(_spawner: Spawner, stack: NetworkStack, sender: Sender<'static, CriticalSectionRawMutex, ReaperStatus<MAX_TRACK_COUNT>, MAX_MESSAGE_COUNT>) -> Result<()> {
    info!("the actual app logic task is starting");
    // graphics_demo(delay, display)?;
    info!("creating a client state");
    let client_state = TcpClientState::<3, IO_BUFFER_SIZE, IO_BUFFER_SIZE>::new();
    info!("creating a tcp client");
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

use embassy_executor::Executor;

static mut CORE1_STACK: embassy_rp::multicore::Stack<{ 1024 * 4 }> = embassy_rp::multicore::Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn embassy_main(spawner: Spawner, wifi_setup_context: SetupWifiContext) {
    info!("embassy is booting up");
    debug_env!(ESP_WIFI_SSID);
    debug_env!(ESP_WIFI_PASSWORD);
    debug_env!(ESP_REAPER_BASE_URL);
    let stack = setup_wifi(spawner, wifi_setup_context)
        .await
        .expect("failed to setup network stack");
    info!("wifi setup correctly, starting the main task");
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
