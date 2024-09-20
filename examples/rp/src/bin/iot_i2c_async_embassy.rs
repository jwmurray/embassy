//! This example shows how to communicate asynchronous using i2c with external chip.
//!
//! It's using embassy's functions directly instead of traits from embedded_hal_async::i2c::I2c.
//! While most of i2c devices are addressed using 7 bits, an extension allows 10 bits too.

#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_imports)]

use defmt::*;
use embassy_executor::Spawner;

// use embassy_rp::i2c::InterruptHandler;
use {defmt_rtt as _, panic_probe as _};

use embassy_rp::{
    bind_interrupts,
    i2c::{self, Config, InterruptHandler},
    peripherals::I2C0,
};

use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use crate::dht20::{initialize, read_temperature_and_humidity};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
});

/// DHT20 sensor: datasheet: https://cdn.sparkfun.com/assets/8/a/1/5/0/DHT20.pdf
mod dht20 {
    use defmt::debug;
    use embassy_rp::{
        i2c::{Async, I2c},
        peripherals::I2C0,
    };
    use embassy_time::Timer;
    use embedded_hal_async::i2c::I2c as I2cAsync;

    const DHT20_I2C_ADDR: u8 = 0x38;
    const DHT20_GET_STATUS: u8 = 0x71;
    const DHT20_READ_DATA: [u8; 3] = [0xAC, 0x33, 0x00];

    const DIVISOR: f32 = 2u32.pow(20) as f32;
    const TEMP_DIVISOR: f32 = DIVISOR / 200.0;

    pub async fn initialize(i2c: &mut I2c<'static, I2C0, Async>) -> bool {
        Timer::after_millis(100).await;
        let mut data = [0x0; 1];
        i2c.write_read(DHT20_I2C_ADDR, &[DHT20_GET_STATUS], &mut data)
            .await
            .expect("Can not read status");

        data[0] & 0x18 == 0x18
    }

    async fn read_data(i2c: &mut I2c<'static, I2C0, Async>) -> [u8; 6] {
        let mut data = [0x0; 6];

        for _ in 0..10 {
            i2c.write(DHT20_I2C_ADDR, &DHT20_READ_DATA)
                .await
                .expect("Can not write data");
            Timer::after_millis(80).await;

            i2c.read(DHT20_I2C_ADDR, &mut data).await.expect("Can not read data");

            if data[0] >> 7 == 0 {
                break;
            }
        }

        data
    }

    pub async fn read_temperature_and_humidity(i2c: &mut I2c<'static, I2C0, Async>) -> (f32, f32) {
        let data = read_data(i2c).await;
        debug!("data = {:?}", data);

        let raw_hum_data = ((data[1] as u32) << 12) + ((data[2] as u32) << 4) + (((data[3] & 0xf0) >> 4) as u32);
        debug!("raw_humidity_data = {:x}", raw_hum_data);
        let humidity = (raw_hum_data as f32) / DIVISOR * 100.0;

        let raw_temp_data = (((data[3] as u32) & 0xf) << 16) + ((data[4] as u32) << 8) + (data[5] as u32);
        debug!("raw_temperature_data = {:x}", raw_temp_data);
        let temperature = (raw_temp_data as f32) / TEMP_DIVISOR - 50.0;

        (temperature, humidity)
    }
}

// Our anonymous hypotetical temperature sensor could be:
// a 12-bit sensor, with 100ms startup time, range of -40*C - 125*C, and precision 0.25*C
// It requires no configuration or calibration, works with all i2c bus speeds,
// never stretches clock or does anything complicated. Replies with one u16.
// It requires only one write to take it out of suspend mode, and stays on.
// Often result would be just on 12 bits, but here we'll simplify it to 16.

enum UncomplicatedSensorId {
    A(UncomplicatedSensorU8),
    B(UncomplicatedSensorU16),
}
enum UncomplicatedSensorU8 {
    First = 0x48,
}
enum UncomplicatedSensorU16 {
    Other = 0x0049,
}

impl Into<u16> for UncomplicatedSensorU16 {
    fn into(self) -> u16 {
        self as u16
    }
}
impl Into<u16> for UncomplicatedSensorU8 {
    fn into(self) -> u16 {
        0x48
    }
}
impl From<UncomplicatedSensorId> for u16 {
    fn from(t: UncomplicatedSensorId) -> Self {
        match t {
            UncomplicatedSensorId::A(x) => x.into(),
            UncomplicatedSensorId::B(x) => x.into(),
        }
    }
}

// embassy_rp::bind_interrupts!(struct Irqs {
//     I2C1_IRQ => InterruptHandler<embassy_rp::peripherals::I2C1>;
// });

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let sda = p.PIN_0;
    let scl = p.PIN_1;

    info!("set up i2c ");
    let mut i2c = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, Config::default());
    let ready = initialize(&mut i2c).await;
    info!("Ready: {}", ready);

    loop {
        let (temperature, humidity) = read_temperature_and_humidity(&mut i2c).await;
        info!("temperature = {}C", temperature);
        info!("humidity = {}%", humidity);

        Timer::after_millis(500).await;
    }
}
