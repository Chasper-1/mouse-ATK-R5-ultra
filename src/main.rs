use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const VID: u16 = 0x373E;
const ATTEMPTS: usize = 10;
const RETRY_DELAY: Duration = Duration::from_millis(500);
const CACHE_FILE: &str = "attack-shark-r5-battery";

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

fn cache_path() -> Option<PathBuf> {
    let dir = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| Path::new(&h).join(".cache")))
        .ok()?;
    Some(dir.join(CACHE_FILE))
}

fn read_cache(path: &Path) -> Option<u8> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn write_cache(path: &Path, val: u8) -> std::io::Result<()> {
    std::fs::write(path, val.to_string())
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

    let out = match battery {
        Some(val) => {
            if let Some(path) = cache_path() {
                let _ = write_cache(&path, val);
            }
            WaybarOutput {
                text: format!("{}%", val),
                tooltip: format!("Attack Shark R5\nЗаряд: {}%", val),
                class: battery_class(val).map(String::from),
                percentage: val,
            }
        }
        None => {
            let cached = cache_path().and_then(|p| read_cache(&p));
            match cached {
                Some(val) => WaybarOutput {
                    text: format!("{}%", val),
                    tooltip: "Attack Shark R5\nНет ответа (данные устарели)".into(),
                    class: Some("off".into()),
                    percentage: val,
                },
                None => WaybarOutput {
                    text: "?%".into(),
                    tooltip: "Attack Shark R5\nНет ответа".into(),
                    class: Some("off".into()),
                    percentage: 0,
                },
            }
        }
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

    #[test]
    fn cache_roundtrip() {
        let path =
            std::env::temp_dir().join(format!("mouse-cache-test-{}.tmp", std::process::id()));
        assert_eq!(read_cache(&path), None);
        write_cache(&path, 54).unwrap();
        assert_eq!(read_cache(&path), Some(54));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn cache_garbage_is_none() {
        let path =
            std::env::temp_dir().join(format!("mouse-cache-garbage-{}.tmp", std::process::id()));
        std::fs::write(&path, "abc\n").unwrap();
        assert_eq!(read_cache(&path), None);
        std::fs::remove_file(&path).unwrap();
    }
}
