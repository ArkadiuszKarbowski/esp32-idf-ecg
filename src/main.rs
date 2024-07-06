use std::sync::Arc;

use esp32_nimble::{
    enums::*,
    utilities::{mutex::Mutex, BleUuid},
    BLEAdvertisementData, BLEDevice, NimbleProperties,
};

use log::info;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let mut adc = AdcDriver::new(peripherals.adc1, &Config::new().calibration(true))?;
    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
        AdcChannelDriver::new(peripherals.pins.gpio36)?;

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

    let secure_characteristic = service.lock().create_characteristic(
        BleUuid::Uuid16(0x1235),
        NimbleProperties::READ | NimbleProperties::READ_ENC | NimbleProperties::READ_AUTHOR,
    );

    secure_characteristic
        .lock()
        .set_value("try to bond".as_bytes());

    let allow_notify = Arc::new(Mutex::new(false));
    let allow_notify_clone = Arc::clone(&allow_notify);

    let notify_characteristic = service
        .lock()
        .create_characteristic(BleUuid::Uuid16(0x1234), NimbleProperties::NOTIFY);
    notify_characteristic
        .lock()
        .on_subscribe(move |_characteristic, conn_desc, sub_event| {
            // Sprawdź, czy urządzenie jest sparowane
            let mut allow_notify = allow_notify_clone.lock();

            if conn_desc.bonded() {
                println!("Subscribed: {:?}", sub_event);
                *allow_notify = true;
            } else {
                println!("Not bonded");
                *allow_notify = false;
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
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        let allow_notify = allow_notify.lock();
        if *allow_notify {
            let value = adc.read(&mut adc_pin)?;
            notify_characteristic
                .lock()
                .set_value(&value.to_le_bytes())
                .notify();
        }
    }
}
