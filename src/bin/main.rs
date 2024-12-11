#![no_std]
#![no_main]


use core::cell::RefCell;

use esp_backtrace as _;
use critical_section::Mutex;
use esp_hal::{
    analog::adc::{
        Adc, 
        AdcConfig, 
        Attenuation}, gpio::{
        Event,
        Input,
        Io,
        Level,
        Output,
        Pull,
    }, prelude::*, time::{
        now, 
        Instant},};
use esp_println::println;

static DETECT_FLAG: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));
static DETECT_FLAG_T: Mutex<RefCell<Instant>> = Mutex::new(RefCell::new(Instant::from_ticks(1)));



#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config: esp_hal::Config = esp_hal::Config::default();
        // Configure the CPU to run at the maximum frequency.
        config.cpu_clock = CpuClock::max();
        config
    });

    //let mut io = Io::new(peripherals.IO_MUX);
    //io.set_interrupt_handler(handler);
    
    let button = peripherals.GPIO19;
    let output_pin = peripherals.GPIO12;

    let mut button = Input::new(button, Pull::Up);
    let mut button_state: Level = Level::High;
    let mut output_pin = Output::new(output_pin, Level::Low);

   /*critical_section::with(|cs| {
        button.listen(Event::RisingEdge);
        BUTTON.borrow_ref_mut(cs).replace(button)
    });*/ 

    let analog_pin = peripherals.GPIO39;
    let mut adc1_config = AdcConfig::new();
    let mut pin = adc1_config.enable_pin(
        analog_pin,
        Attenuation::Attenuation11dB,
    );
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);
    
    let duration_t = 500; //Milis
    let duration_t = duration_t * 1_000;
    let duration_t = Instant::from_ticks(duration_t);
    let mut time_start = now();
   
    loop {
        let zero_cross: bool = critical_section::with(|cs| *DETECT_FLAG.borrow_ref(cs));
        let zero_cross_time = critical_section::with(|cs| *DETECT_FLAG_T.borrow_ref(cs));

        let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
        let pin_value: f32 = ((pin_value as f32) * 100.0)/3750.0;

        // Define los conjuntos difusos
        let low = FuzzySet { name: "Low", a: 0.0, b: 0.0, c: 20.0, d: 50.0 };
        let medium = FuzzySet { name: "Medium", a: 20.0, b: 50.0, c: 50.0, d: 75.0 };
        let high = FuzzySet { name: "High", a: 50.0, b: 75.0, c: 100.0, d: 100.0 };
        
        // Define las reglas como un array de tamaño fijo
        let rules: [Rule; 3] = [
            Rule { fuzzy_set: low, output: 165.0 },
            Rule { fuzzy_set: medium, output: 90.0 },
            Rule { fuzzy_set: high, output: 15.0 },
        ];

        let results = apply_rules(pin_value, &rules);

        let output: f32 = defuzzify(&results);

        let signal_delay = (output as u64) * 8333 / 180;
        let signal_delay = Instant::from_ticks(signal_delay as u64);

        let time_now = now();

        if button_state != button.level() {
            if button.is_high(){
                if time_now.ticks() - zero_cross_time.ticks() >= signal_delay.ticks(){
                    output_pin.set_high();
                }    
    
                if time_now.ticks() - zero_cross_time.ticks() >= signal_delay.ticks() + Instant::from_ticks(500).ticks(){    
                    output_pin.set_low();
                    critical_section::with(|cs|{
                        let mut zero_cross = DETECT_FLAG.borrow_ref_mut(cs);
                        * zero_cross = false;
                    });
                }
            }
            button_state = button.level();
        }
    
        if time_now.ticks() - time_start.ticks() >= duration_t.ticks(){
            println!("Salida: {}",output);
            time_start = now();
        }
    }
}

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
        165.0
    } else {
        numerator / denominator
    }
}

/*
#[handler]
#[ram]
fn handler() {
    println!("Handler");
    critical_section::with(|cs|{
        let mut zero_cross = DETECT_FLAG.borrow_ref_mut(cs);
        * zero_cross = true;
    });
    critical_section::with(|cs|{
        let mut zero_cross_time = DETECT_FLAG_T.borrow_ref_mut(cs);
        * zero_cross_time = now();
    });

    critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt()
    });
}*/