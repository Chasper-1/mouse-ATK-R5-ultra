use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::time::Duration;

const VID: u16 = 0x373E;
const ATTEMPTS: usize = 10;
const RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    percentage: u8,
}

/// Порт JS getBatPer(): feature-запрос батареи.
/// Новый протокол: resp[2]=0xA1, resp[5]=2, resp[7]=131 -> процент в resp[9].
/// Старый протокол: всё сдвинуто на байт влево -> процент в resp[8].
fn get_battery(dev: &HidDevice) -> Option<u8> {
    let mut req = [0u8; 65];
    req[3] = 2; // deviceId: мышь
    req[4] = 2; // группа "параметры"
    req[6] = 131; // 0x83 = запрос батареи
    dev.send_feature_report(&req).ok()?;
    std::thread::sleep(Duration::from_millis(100));
    let mut resp = [0u8; 65];
    resp[0] = 0;
    dev.get_feature_report(&mut resp).ok()?;
    if resp[2] == 0xA1 && resp[5] == 2 && resp[7] == 131 {
        return Some(resp[9]);
    }
    if resp[1] == 0xA1 && resp[4] == 2 && resp[6] == 131 {
        return Some(resp[8]);
    }
    None
}

/// Класс для waybar по уровню заряда. Обычный режим — класса нет.
fn battery_class(val: u8) -> Option<&'static str> {
    if val < 5 {
        Some("critical")
    } else if val <= 20 {
        Some("low")
    } else {
        None
    }
}

fn main() {
    let mut api = HidApi::new().expect("HID API init");

    // До ATTEMPTS попыток достучаться до мыши
    let mut battery = None;
    'outer: for _ in 0..ATTEMPTS {
        let _ = api.refresh_devices();
        for info in api.device_list() {
            if info.vendor_id() != VID {
                continue;
            }
            if let Ok(dev) = info.open_device(&api)
                && let Some(pct) = get_battery(&dev)
            {
                battery = Some(pct);
                break 'outer;
            }
        }
        std::thread::sleep(RETRY_DELAY);
    }

    let Some(val) = battery else {
        return; // мышь не ответила — ничего не выводим
    };
    let out = WaybarOutput {
        text: format!("{}%", val),
        tooltip: format!("Attack Shark R5\nЗаряд: {}%", val),
        class: battery_class(val).map(String::from),
        percentage: val,
    };
    println!("{}", serde_json::to_string(&out).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_class_boundaries() {
        assert_eq!(battery_class(4), Some("critical"));
        assert_eq!(battery_class(5), Some("low"));
        assert_eq!(battery_class(20), Some("low"));
        assert_eq!(battery_class(21), None);
        assert_eq!(battery_class(100), None);
    }
}
