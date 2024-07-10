use esp_idf_svc::hal::{
    adc::{AdcChannelDriver, AdcDriver},
    peripherals::Peripherals,
    adc::config::Config,
    adc::attenuation,
};
use std::sync::mpsc::SyncSender;

pub fn adc_read(sender: &SyncSender<u16>) -> anyhow::Result<()> {
    let peripherals = match Peripherals::take() {
        Ok(peripherals) => peripherals,
        Err(e) => {
            return Err(e.into());
        }
    };
    let mut adc = match AdcDriver::new(peripherals.adc1, &Config::new().calibration(true)) {
        Ok(adc) => adc,
        Err(e) => {
            return Err(e.into());
        }
    };
    let mut adc_pin: AdcChannelDriver<{ attenuation::DB_11 }, _> =
        match AdcChannelDriver::new(peripherals.pins.gpio36) {
            Ok(adc_pin) => adc_pin,
            Err(e) => {
                return Err(e.into());
            }
        };

    loop {
        esp_idf_svc::hal::delay::FreeRtos::delay_ms(500);

        let value = match adc.read(&mut adc_pin) {
            Ok(value) => value,
            Err(e) => {
                return Err(e.into());
            }
        };
        sender.send(value).unwrap();
    }
}