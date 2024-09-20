#![no_std]
#![no_main]

/// This is a simple example that demonstrates how to use the `embassy` crate to blink an LED on
/// the Raspberry Pi Pico board. The LED is toggled by two tasks with slightly different periods,
/// leading to the apparent duty cycle of the LED increasing, then decreasing, linearly. The phenomenon
/// is similar to interference and the 'beats' you can hear if you play two frequencies close to one another
///    [Link explaining it](https://www.physicsclassroom.com/class/sound/Lesson-3/Interference-and-Beats)
///
/// The LED is connected to pin 21.
///
/// A third task prints a message every second on the debug port.
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embassy_time::{Duration, Ticker};
use gpio::{AnyPin, Level, Output};
use {defmt_rtt as _, panic_probe as _};
use {defmt_rtt as _, panic_probe as _};

type LedType = Mutex<ThreadModeRawMutex, Option<Output<'static>>>;
static LED: LedType = Mutex::new(None);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals: embassy_rp::Peripherals = embassy_rp::init(Default::default());
    // set the content of the global LED reference to the real LED pin
    let led = Output::new(AnyPin::from(peripherals.PIN_21), Level::High);
    // inner scope is so that once the mutex is written to, the MutexGuard is dropped, thus the
    // Mutex is released
    {
        *(LED.lock().await) = Some(led);
    }
    let dt = 100 * 1_000_000;
    let k = 1.003;

    unwrap!(spawner.spawn(hello_debug(1)));
    unwrap!(spawner.spawn(toggle_led(&LED, Duration::from_nanos(dt))));
    unwrap!(spawner.spawn(toggle_led(&LED, Duration::from_nanos((dt as f64 * k) as u64))));
}

#[embassy_executor::task(pool_size = 1)]
async fn hello_debug(delay_secs: u64) {
    let mut count = 0;
    loop {
        info!("{}. Hello Pico World!", count);
        Timer::after_secs(delay_secs).await;
        count += 1;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn toggle_led(led: &'static LedType, delay: Duration) {
    let mut ticker = Ticker::every(delay);
    loop {
        {
            let mut led_unlocked = led.lock().await;
            if let Some(pin_ref) = led_unlocked.as_mut() {
                pin_ref.toggle();
            }
        }
        ticker.next().await;
    }
}
