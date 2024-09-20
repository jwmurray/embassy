#![no_std]
#![no_main]
///
/// Connect a potentiometer to pin 26 and an LED to pin 16.
/// The LED will turn on when the potentiometer value is greater than 2048.
/// The potentiometer value is read every 100ms and the average of 10 readings is sent to the main
/// thread every second.
/// The main thread will print the average value and turn the LED on or off based on the value.
/// The potentiometer value is between 0 and 4095.
///
/// This example demonstrates how to use the ADC peripheral to read an analog value from a pin.
/// The ADC peripheral is used in async mode to read the value from the potentiometer pin.
///
/// Note that we use the `embassy_sync` crate to create a channel to send the ADC value to the main thread.
/// The read_adc_value thread sends the average of 10 readings to the main thread every second.
/// The main thread will print the average value and turn the LED on or off based on the value.
///
/// We could use the value to variably control the frequency of blinks on the LED.
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Async, Channel, Config, InterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

// static CHANNEL: SyncChannel<ThreadModeRawMutex, u16, 64> = SyncChannel::new();
// static DURATION_CHANNEL: SyncChannel<ThreadModeRawMutex, u64, 64> = SyncChannel::new();
static CHANNEL: Signal<ThreadModeRawMutex, u16> = Signal::new();
static DURATION_SIGNAL: Signal<ThreadModeRawMutex, u64> = Signal::new();

#[embassy_executor::task]
async fn toggle_led(mut led: Output<'static>) {
    info!("toggle_led task started");
    loop {
        info!("Waiting to receive duration...");
        let duration = DURATION_SIGNAL.wait().await;
        info!("Received duration: {}", duration);

        // Toggle the LED state
        led.toggle();
        info!("LED toggled");

        // Wait for the specified duration
        Timer::after(Duration::from_millis(duration)).await;
    }
}

#[embassy_executor::task]
async fn read_adc_value(mut adc: Adc<'static, Async>, mut p26: Channel<'static>) {
    info!("read_adc_value task started");
    loop {
        // Simulate reading ADC value and sending it
        let value = adc.read(&mut p26).await.unwrap();
        info!("Sending ADC value: {}", value);
        CHANNEL.signal(value);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let led = Output::new(p.PIN_16, Level::Low);

    info!("Setting up ADC");
    let adc = Adc::new(p.ADC, Irqs, Config::default());
    let p26 = Channel::new_pin(p.PIN_26, Pull::None);

    // Spawn the task that reads the ADC value
    spawner.spawn(read_adc_value(adc, p26)).unwrap();

    // Spawn the task that toggles the LED
    spawner.spawn(toggle_led(led)).unwrap();
    use core::cmp::max;
    use core::cmp::min;
    loop {
        let value = CHANNEL.wait().await as u64;
        info!("ADC value: {}", value);

        // Calculate the duration based on the ADC value

        // let duration = if value > 2048 {
        //     500 // 500 ms
        // } else {
        //     1000 // 1000 ms
        // };

        let duration = (5000.0 - (value as f64 / 4095.0) * 5000.0) as i64;
        let duration: u64 = max(min(5000_i64, duration as i64), 100) as u64;

        info!("Sending duration: {}", duration);
        DURATION_SIGNAL.signal(duration);
        info!("Duration sent");
    }
}
