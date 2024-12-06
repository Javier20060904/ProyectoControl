#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation}, delay::Delay, prelude::*
};
use log::info;

const SET_POINT: f32 = 2.5;

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config: esp_hal::Config = esp_hal::Config::default();
        // Configure the CPU to run at the maximum frequency.
        config.cpu_clock = CpuClock::max();
        config
    });

    let analog_pin = peripherals.GPIO4;
    let mut adc2_config = AdcConfig::new();
    let mut pin = adc2_config.enable_pin(
        analog_pin,
        Attenuation::Attenuation11dB,
    );
    let mut adc1 = Adc::new(peripherals.ADC2, adc2_config);
    
    let delay = Delay::new();
    
    loop {
        let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
        let pin_value: f32 = ((pin_value as f32) * 3.3)/4095.0;
        let output: f32 = pin_value - (pin_value-SET_POINT);

        info!("Input: {}", pin_value);
        info!("Error: {}", (pin_value-SET_POINT));
        info!("Salida: {}", output);
        delay.delay(500.millis());
    }
}