use esp32_nimble::{
    enums::*,
    utilities::{mutex::Mutex, BleUuid},
    BLEAdvertisementData, BLEDevice, NimbleProperties, NimbleSub,
};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use esp_idf_svc::hal::{
    adc::{attenuation, config::Config, AdcChannelDriver, AdcDriver},
    cpu::Core,
    peripherals::Peripherals,
};

use esp_idf_svc::log::EspLogger;
use log::{debug, error, info};

pub mod adc_reader;
pub mod thread;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    let logger = EspLogger; // This is a workaround for the logger not being able to be used in the closure below
    logger.set_target_level("adc", log::LevelFilter::Debug)?;
    debug!(target:"adc", "Starting BLE server");

    let peripherals = match Peripherals::take() {
        Ok(peripherals) => peripherals,
        Err(e) => {
            debug!(target:"adc", "Failed to take peripherals: {:?}", e);
            return Err(e.into());
        }
    };

    let adc = match AdcDriver::new(peripherals.adc1, &Config::new().calibration(true)) {
        Ok(adc) => adc,
        Err(e) => {
            debug!(target:"adc", "Failed to initialize ADC: {:?}", e);
            return Err(e.into());
        }
    };

    let adc_pin: AdcChannelDriver<{ attenuation::DB_11 }, _> =
        match AdcChannelDriver::new(peripherals.pins.gpio36) {
            Ok(adc_pin) => adc_pin,
            Err(e) => {
                debug!(target:"adc", "Failed to initialize ADC pin: {:?}", e);
                return Err(e.into());
            }
        };

    let adc = Arc::new(Mutex::new(adc));
    let adc_pin = Arc::new(Mutex::new(adc_pin));

    let (sender, receiver) = mpsc::sync_channel::<u16>(20);
    let sender = Arc::new(Mutex::new(sender));
    let receiver = Arc::new(Mutex::new(receiver));

    let ble_device = BLEDevice::take();
    let ble_advertiser = ble_device.get_advertising();
    ble_device
        .security()
        .set_auth(AuthReq::all())
        .set_passkey(002137)
        .set_io_cap(SecurityIOCap::DisplayOnly)
        .resolve_rpa();

    let server = ble_device.get_server();

    server.on_connect(|server, client_desc| {
        info!("Client connected: {:?}", client_desc);

        if server.connected_count() > 1 {
            info!("Disconnecting client");
            server.disconnect(client_desc.conn_handle()).unwrap();
        }
        if !client_desc.bonded() {
            info!("Client not bonded");
        }
    });

    server.on_disconnect(|_desc, reason| {
        info!("Client disconnected ({:?})", reason);
    });

    server.on_authentication_complete(|client_desc, result| {
        info!("AuthenticationComplete({:?}): {:?}", result, client_desc);
    });

    let service = server.create_service(BleUuid::Uuid16(0xABCD));

    let start_measurement_characteristic = service.lock().create_characteristic(
        BleUuid::Uuid16(0x1235),
        NimbleProperties::WRITE | NimbleProperties::WRITE_ENC | NimbleProperties::WRITE_AUTHOR,
    );
    start_measurement_characteristic.lock().on_write(|args| {
        info!(
            "Wrote to writable characteristic: {:?} -> {:?}",
            std::str::from_utf8(args.current_data()).unwrap(),
            args.recv_data().to_ascii_lowercase()
        );
    });

    let notify_characteristic = service
        .lock()
        .create_characteristic(BleUuid::Uuid16(0x1234), NimbleProperties::NOTIFY);

    let running = Arc::new(AtomicBool::new(false));

    notify_characteristic
        .lock()
        .on_subscribe(move |_characteristic, conn_desc, sub: NimbleSub| {
            if conn_desc.bonded() {
                if !sub.is_empty() {
                    let sender_clone = Arc::clone(&sender);
                    let running_clone = Arc::clone(&running);
                    let adc_clone = Arc::clone(&adc);
                    let adc_pin_clone = Arc::clone(&adc_pin);

                    running_clone.store(true, Ordering::Relaxed);

                    let _worker = thread::spawn(Core::Core1, move || {
                        let sender_guard = sender_clone.lock();
                        let mut adc_guard = adc_clone.lock();
                        let mut adc_pin_guard = adc_pin_clone.lock();
                        if let Err(e) = adc_reader::adc_read(
                            &*sender_guard,
                            running_clone,
                            &mut *adc_guard,
                            &mut *adc_pin_guard,
                        ) {
                            error!("Error reading ADC: {:?}", e);
                        }
                    });

                    debug!(target: "adc", "Client subscribed with flags: {:?}", sub);
                } else {
                    // Empty means unsubscribed
                    running.store(false, Ordering::Relaxed);
                    debug!(target: "adc", "Client unsubscribed");
                }
            }
        });

    ble_advertiser.lock().set_data(
        BLEAdvertisementData::new()
            .name("ECG-Device")
            .add_service_uuid(BleUuid::Uuid16(0xABCD)),
    )?;
    ble_advertiser.lock().start()?;

    info!("bonded_addresses: {:?}", ble_device.bonded_addresses());

    loop {
        esp_idf_svc::hal::delay::FreeRtos::delay_ms(400);
        let receiver_guard = receiver.lock();
        let value = receiver_guard.recv().unwrap();
        info!("Received value: {}", value);

        notify_characteristic
            .lock()
            .set_value(&value.to_le_bytes())
            .notify();
    }
}
