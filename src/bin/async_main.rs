#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::AnyPin;
use esp_hal::peripheral::Peripheral;
use esp_hal::uart::AnyUart;
use esp_hal::{clock::CpuClock, Async};
use log::{info, warn};

use hc_12::{BaudRate, Hc12};

async fn setup_hc12<'d>(
    uart: impl Peripheral<P = esp_hal::uart::AnyUart> + 'd,
    rx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
    tx: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
    set: impl Peripheral<P = esp_hal::gpio::AnyPin> + 'd,
) {
    info!("Initializing hc-12");
    let mut hc_12: Hc12<'_, Async> = Hc12::<'_, Async>::new(
        uart.map_into::<AnyUart>(),
        rx.map_into::<AnyPin>(),
        tx.map_into::<AnyPin>(),
        set.map_into::<AnyPin>(),
    )
    .await
    .unwrap();

    info!("Setting auto baud");
    let detected_baud = hc_12.auto_baud().await.unwrap();
    info!("Detected baud: {}", u32::from(detected_baud));

    info!("Setting baud");
    hc_12.set_baud(&BaudRate::Baud115200).await.unwrap();
    let detected_baud = hc_12.auto_baud().await.unwrap();
    info!("Detected baud: {}", u32::from(detected_baud));

    info!("Resetting hc-12");
    hc_12.set_default().await.unwrap();
    hc_12.auto_baud().await.unwrap();

    info!("Setting transmission mode");
    hc_12
        .set_transmission_mode(&hc_12::TransmissionMode::Fu2)
        .await
        .unwrap();

    info!("Done");

    for _ in 0..5 {
        Timer::after(Duration::from_secs(1)).await;
        if hc_12.test().await.is_ok() {
            info!("Ok");
        } else {
            log::error!("Error");
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut uart = peripherals.UART1.into_ref();
    let mut rx = peripherals.GPIO20.into_ref();
    let mut tx = peripherals.GPIO21.into_ref();
    let mut set = peripherals.GPIO10.into_ref();

    setup_hc12(
        uart.reborrow().map_into::<AnyUart>(),
        rx.reborrow().map_into::<AnyPin>(),
        tx.reborrow().map_into::<AnyPin>(),
        set.reborrow().map_into::<AnyPin>(),
    )
    .await;

    Timer::after_secs(3).await;

    setup_hc12(
        uart.reborrow().map_into::<AnyUart>(),
        rx.reborrow().map_into::<AnyPin>(),
        tx.reborrow().map_into::<AnyPin>(),
        set.reborrow().map_into::<AnyPin>(),
    )
    .await;

    warn!("Goodbye");

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}
