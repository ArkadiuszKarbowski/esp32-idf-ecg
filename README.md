# esp32-idf-ecg

This project is designed for the ESP32 microcontroller using the ESP-IDF framework. 
It is part of a larger project aimed at creating an ECG monitoring device integrated into a car's steering wheel.
The current prototype operates on the ESP32-WROOM module and utilizes the AD8232 sensor. 
This setup serves as the initial development platform while a dedicated electronic circuit is still under development.
It demonstrates how to set up a BLE (Bluetooth Low Energy) device that reads data from an ADC (Analog-to-Digital Converter) and sends it to a connected client. 
The project leverages various libraries such as `esp32_nimble` for BLE functionalities, `log` for logging, and `esp_idf_svc` for system services.

## Features

- **BLE Peripheral**: The device advertises itself as "ECG-Device" and provides a BLE service with multiple characteristics.
- **Security**: Implements BLE security with authentication and bonding.
- **ADC Reading**: Reads data from the ADC and sends it to the BLE client.
- **Multithreading**: Utilizes multiple cores of the ESP32 to handle BLE events and ADC reading concurrently.

## Project Structure

- `src/`
  - `main.rs`: The main entry point of the application.
  - `adc_reader.rs`: Module responsible for reading data from the ADC.
  - `thread.rs`: Module for handling thread operations.

## Getting Started

### Prerequisites

- ESP32 development board and AD8232
- ESP-IDF environment set up on your machine
- Rust and cargo installed

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/ArkadiuszKarbowski/esp32-idf-ecg.git
   cd ECG-Device
   ```

2. Build and flash the project to your ESP32:
    ```bash
    cargo run
    ```
License

This project is licensed under the MIT License - see the LICENSE file for details.
