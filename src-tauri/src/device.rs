use nusb::MaybeFuture;
use serde::Serialize;

const ST_VID: u16 = 0x0483;
const DFU_PID: u16 = 0xDF11;
const CDC_PID: u16 = 0x5740;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchDevice {
    pub state: String,
    pub vendor_id: String,
    pub product_id: String,
    pub product: Option<String>,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub topology_path: String,
    pub device_address: u8,
    pub serial_port: Option<String>,
}

pub fn scan_devices() -> Result<Vec<TouchDevice>, String> {
    let serial_ports = serialport::available_ports().unwrap_or_default();
    let devices = nusb::list_devices()
        .wait()
        .map_err(|error| error.to_string())?;

    Ok(devices
        .filter(|device| {
            device.vendor_id() == ST_VID
                && (device.product_id() == DFU_PID || device.product_id() == CDC_PID)
        })
        .map(|device| {
            let product = device.product_string().map(ToOwned::to_owned);
            let state = classify_device_state(device.product_id(), product.as_deref());
            let serial_port = serial_ports.iter().find_map(|port| match &port.port_type {
                serialport::SerialPortType::UsbPort(info)
                    if info.vid == device.vendor_id()
                        && info.pid == device.product_id()
                        && serials_compatible(
                            info.serial_number.as_deref(),
                            device.serial_number(),
                        ) =>
                {
                    Some(port.port_name.clone())
                }
                _ => None,
            });
            let chain = device
                .port_chain()
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(".");

            TouchDevice {
                state,
                vendor_id: format!("{:04X}", device.vendor_id()),
                product_id: format!("{:04X}", device.product_id()),
                product,
                manufacturer: device.manufacturer_string().map(ToOwned::to_owned),
                serial_number: device.serial_number().map(ToOwned::to_owned),
                topology_path: format!("{}-{chain}", device.bus_id()),
                device_address: device.device_address(),
                serial_port,
            }
        })
        .collect())
}

pub fn runtime_is_present() -> bool {
    scan_devices().is_ok_and(|devices| devices.iter().any(|device| device.state == "runtime"))
}

fn classify_device_state(product_id: u16, product: Option<&str>) -> String {
    if product_id == CDC_PID {
        return "runtime".into();
    }

    let normalized = product.unwrap_or_default().to_ascii_lowercase();
    if normalized.contains("daisy") && normalized.contains("boot") {
        "daisy_bootloader".into()
    } else if normalized.contains("dfu") || normalized.contains("stm") {
        "stm_rom_dfu".into()
    } else {
        "dfu_unknown".into()
    }
}

fn serials_compatible(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => true,
    }
}
