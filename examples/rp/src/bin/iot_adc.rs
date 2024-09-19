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
use embassy_rp::adc::{Adc, Async, Config, InterruptHandler};
use embassy_rp::gpio;
use embassy_rp::gpio::Pull;
use embassy_rp::{adc, bind_interrupts};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::Timer;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

static CHANNEL: Channel<ThreadModeRawMutex, u16, 64> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_16, Level::Low);

    info!("Setting up ADC");
    let adc = Adc::new(p.ADC, Irqs, Config::default());
    let p26 = adc::Channel::new_pin(p.PIN_26, Pull::None);

    // spawn the task that reads the ADC value
    spawner
        .spawn(read_adc_value(adc, p26, CHANNEL.sender()))
        .unwrap();

    let rx_adv_value = CHANNEL.receiver();

    loop {
        let value = rx_adv_value.receive().await;
        // we should get a new value every 1s
        // the value we are getting will be somewhere between 0 and 4095

        info!("ADC value: {}", value);

        if value > 2048 {
            led.set_high();
        } else {
            led.set_low();
        }
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn read_adc_value(
    mut adc: Adc<'static, Async>,
    mut p26: adc::Channel<'static>,
    tx_value: Sender<'static, ThreadModeRawMutex, u16, 64>,
) {
    let mut measurements = [0u16; 10];
    let mut pos = 0;

    loop {
        measurements[pos] = adc.read(&mut p26).await.unwrap();
        pos = (pos + 1) % 10;

        if pos == 0 {
            // compute average of measurements
            let average = measurements.iter().sum::<u16>() / 10;

            // send average to main thread
            tx_value.send(average).await;
        }

        Timer::after_millis(100).await;
    }
}