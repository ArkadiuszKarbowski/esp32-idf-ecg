use esp_idf_svc::hal::{
    adc::{AdcChannelDriver, AdcDriver, ADC1},
    gpio::Gpio36,
};
use log::debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;


pub fn adc_read(sender: &SyncSender<u16>, running: Arc<AtomicBool>, adc: &mut AdcDriver<ADC1>, adc_pin: &mut AdcChannelDriver<3, Gpio36>) -> anyhow::Result<()> {
    
    while running.load(Ordering::Relaxed) {
        esp_idf_svc::hal::delay::FreeRtos::delay_ms(500);

        match adc.read(adc_pin) {
            Ok(value) => {
                if let Err(e) = sender.send(value) {
                    debug!(target:"adc", "Failed to send ADC value: {:?}", e);
                    break;
                }
            }
            Err(e) => {
                debug!(target:"adc", "Failed to read ADC: {:?}", e);
                return Err(e.into());
            }
        };
    }
    
    debug!(target:"adc", "ADC read loop finished");
    Ok(())
}
