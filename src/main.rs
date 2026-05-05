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
    // Храним по path (строка), чтобы не путать устройства при переподключении
    let mut devices: HashMap<String, HidDevice> = HashMap::new();
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

    let mut buf = [0u8; 64];

    loop {
        // 1. Обновляем список портов (как твой питон сканирует hidraw)
        let _ = api.refresh_devices();
        for device_info in api.device_list() {
            if device_info.vendor_id() == VID && device_info.interface_number() != 0 {
                let path = device_info.path().to_string_lossy().into_owned();

                if !devices.contains_key(&path) {
                    if let Ok(dev) = device_info.open_device(&api) {
                        let _ = dev.write(&h_pkt);
                        std::thread::sleep(Duration::from_millis(50));
                        let _ = dev.write(&b_pkt);
                        devices.insert(path, dev);
                    }
                }
            }
        }

        let mut received_any = false;
        let mut to_remove = Vec::new();

        // 2. Опрашиваем все открытые порты (твой цикл по values)
        for (path, dev) in devices.iter() {
            match dev.read_timeout(&mut buf, 10) {
                Ok(res) if res >= 3 && buf[0] == 0x04 && buf[1] == 0x03 => {
                    let val = buf[2];
                    if val > 0 && val <= 100 {
                        if val != global_last_val {
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
                // Если ошибка чтения (устройство выткнули) — помечаем на удаление
                Err(_) => to_remove.push(path.clone()),
                _ => {}
            }
        }

        // Чистим мертвые дескрипторы
        for path in to_remove {
            devices.remove(&path);
        }

        // 3. Если никто не ответил — пинаем всех и ждем секунду
        if !received_any {
            for dev in devices.values() {
                let _ = dev.write(&b_pkt);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
