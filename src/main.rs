use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

const VID: u16 = 0x373E;

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    class: String,
    percentage: u8,
}

fn calc_crc(data: &[u8]) -> u16 {
    data.iter().take(62).map(|&b| b as u16).sum()
}

fn main() {
    let mut api = HidApi::new().expect("Failed to init HID API");
    let mut devices: HashMap<usize, HidDevice> = HashMap::new();
    let mut global_last_val = 0;

    let mut h_pkt = [0u8; 64];
    h_pkt[0] = 0x08;
    h_pkt[1] = 0x01;
    let h_crc = calc_crc(&h_pkt);
    h_pkt[62] = (h_crc >> 8) as u8;
    h_pkt[63] = (h_crc & 0xFF) as u8;

    let mut b_pkt = [0u8; 64];
    b_pkt[0] = 0x08;
    b_pkt[1] = 0x02;
    b_pkt[2] = 0x02;
    let b_crc = calc_crc(&b_pkt);
    b_pkt[62] = (b_crc >> 8) as u8;
    b_pkt[63] = (b_crc & 0xFF) as u8;

    loop {
        // Если список устройств пуст, пытаемся их найти
        if devices.is_empty() {
            let _ = api.refresh_devices(); // Обновляем список доступных HID устройств
            for device_info in api.device_list() {
                let iface = device_info.interface_number();
                if device_info.vendor_id() == VID && (iface == 4 || iface == 7) {
                    if let Ok(dev) = device_info.open_device(&api) {
                        let id = devices.len();
                        let _ = dev.write(&h_pkt);
                        std::thread::sleep(Duration::from_millis(50));
                        let _ = dev.write(&b_pkt);
                        devices.insert(id, dev);
                    }
                }
            }

            if devices.is_empty() {
                // Если всё еще пусто, ждем 5 секунд и пробуем снова, чтобы не грузить процессор
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        }

        let mut received_any = false;
        let mut to_remove = Vec::new();

        for (&id, dev) in devices.iter() {
            let mut buf = [0u8; 64];
            match dev.read_timeout(&mut buf, 10) {
                Ok(res) => {
                    if res >= 3 && buf[0] == 0x04 && buf[1] == 0x03 {
                        let val = buf[2];
                        if val > 0 && val <= 100 && val != global_last_val {
                            let output = WaybarOutput {
                                text: format!("{}%", val),
                                tooltip: format!("Attack Shark R5\nЗаряд: {}%", val),
                                class: if val < 20 {
                                    "critical".into()
                                } else {
                                    "normal".into()
                                },
                                percentage: val,
                            };
                            println!("{}", serde_json::to_string(&output).unwrap());
                            global_last_val = val;
                        }
                        received_any = true;
                    }
                }
                Err(_) => {
                    // Если устройство отвалилось (выключили), помечаем на удаление
                    to_remove.push(id);
                }
            }
        }

        // Чистим список, если устройства отключились
        for id in to_remove {
            devices.remove(&id);
        }

        if !received_any && !devices.is_empty() {
            for dev in devices.values() {
                let _ = dev.write(&b_pkt);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
