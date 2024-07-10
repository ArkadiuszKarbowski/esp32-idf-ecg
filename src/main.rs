use std::sync::{mpsc, Arc};

use esp32_nimble::{
    enums::*,
    utilities::{mutex::Mutex, BleUuid},
    BLEAdvertisementData, BLEDevice, NimbleProperties,
};

use log::{error, info};
use esp_idf_svc::hal::cpu::Core;

pub mod adc_reader;
pub mod thread;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

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

    notify_characteristic
        .lock()
        .on_subscribe(move |_characteristic, conn_desc, _sub_event| {
            if conn_desc.bonded() {
                let sender = Arc::clone(&sender);
                let _worker = thread::spawn(Core::Core1, move || {
                    let sender_guard = sender.lock();
                    if let Err(e) = adc_reader::adc_read(&*sender_guard) {
                        error!("Error reading ADC: {:?}", e);
                    }
                });
            } else {
                info!("Not bonded");
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
