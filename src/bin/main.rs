#![no_std]
#![no_main]

use core::time::{self, Duration};

use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation}, delay::{self,Delay}, prelude::*, time::{now, Instant}};
use esp_println::print;
use log::{info, log};

fn trapezoidal(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    if x <= a || x >= d {
        0.0
    } else if x < b {
        (x - a) / (b - a)
    } else if x <= c {
        1.0
    } else {
        (d - x) / (d - c)
    }
}

struct FuzzySet {
    name: &'static str,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

impl FuzzySet {
    fn membership(&self, x: f32) -> f32 {
        trapezoidal(x, self.a, self.b, self.c, self.d)
    }
}

struct Rule {
    fuzzy_set: FuzzySet,
    output: f32,
}

fn apply_rules(x: f32, rules: &[Rule; 3]) -> [(f32, f32); 3] {
    let mut results = [(0.0, 0.0); 3];
    for (i, rule) in rules.iter().enumerate() {
        let membership = rule.fuzzy_set.membership(x);
        results[i] = (membership, rule.output);
    }
    results
}

fn defuzzify(results: &[(f32, f32); 3]) -> f32 {
    let numerator: f32 = results.iter().map(|(mu, y)| mu * y).sum();
    let denominator: f32 = results.iter().map(|(mu, _)| mu).sum();
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config: esp_hal::Config = esp_hal::Config::default();
        // Configure the CPU to run at the maximum frequency.
        config.cpu_clock = CpuClock::max();
        config
    });

    let analog_pin = peripherals.GPIO39;
    let mut adc2_config = AdcConfig::new();
    let mut pin = adc2_config.enable_pin(
        analog_pin,
        Attenuation::Attenuation11dB,
    );
    let mut adc1 = Adc::new(peripherals.ADC1, adc2_config);
    
    let duration_t = 500; //Milis
    let duration_t = duration_t * 1_000;
    let duration_t = Instant::from_ticks(duration_t);
    let delay = Delay::new();
    let mut time_start = now();
   
    loop {
        let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
        let pin_value: f32 = ((pin_value as f32) * 100.0)/3750.0;

        // Define los conjuntos difusos
        let low = FuzzySet { name: "Low", a: 0.0, b: 0.0, c: 25.0, d: 50.0 };
        let medium = FuzzySet { name: "Medium", a: 25.0, b: 50.0, c: 50.0, d: 75.0 };
        let high = FuzzySet { name: "High", a: 50.0, b: 75.0, c: 100.0, d: 100.0 };

        // Define las reglas como un array de tamaño fijo
        let rules: [Rule; 3] = [
            Rule { fuzzy_set: low, output: 165.0 },
            Rule { fuzzy_set: medium, output: 90.0 },
            Rule { fuzzy_set: high, output: 15.0 },
        ];

        let results = apply_rules(pin_value, &rules);

        let output: f32 = defuzzify(&results);

        let time_now = now();
        if time_now.ticks() - time_start.ticks() >= duration_t.ticks(){
            info!("{}",output);
            time_start = now();
        }
    }
}