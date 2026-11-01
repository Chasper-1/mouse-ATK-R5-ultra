# mouse-ATK-R5-ultra

Модуль для [waybar](https://github.com/Alexays/Waybar), который показывает уровень заряда мыши **ATTACK SHARK R5 Ultra**.

Программа одноразовая: запустилась, опросила мышь, вывела JSON и завершилась. Периодический опрос выполняет сам waybar через `interval`, поэтому запускать её как сервис не нужно и бессмысленно.

**Релиз на данный момент не актуальный, установку лучше осуществлять сборкой.**

## Требования

- Linux с udev/systemd;
- подключённая мышь R5 Ultra (2.4G-донгл или провод);
- утилиты `sudo` или `pkexec` — только для установки udev-правила, один раз.

## Сборка

```sh
cargo build --release
install -m 755 target/release/mouse-ATK-R5-ultra ~/.local/bin/mouse
```

## Установка правил

Программа читает и пишет feature-отчёты, поэтому ей нужен доступ к `/dev/hidraw*`. По умолчанию эти файлы принадлежат root, поэтому один раз ставится udev-правило — дальше всё работает от обычного пользователя:

```sh
sudo ./install.sh
```

Скрипт создаёт `/etc/udev/rules.d/70-attack-shark.rules`, перезагружает правила udev и применяет их к текущим hidraw-устройствам.

Правило действует через `TAG+="uaccess"` — это ACL для пользователя активной локальной сессии, а не `0666` для всех. Имя файла начинается с `70-`, чтобы правило применялось раньше, чем `73-seat-late.rules` (именно там systemd применяет ACL).

Если на машине уже есть старые правила для этой мыши, их стоит убрать — одной командой:

```sh
sudo rm -f /etc/udev/rules.d/99-mouse.rules /etc/udev/rules.d/99-mouse-webhid.rules && sudo ./install.sh
```

Проверка:

```sh
getfacl /dev/hidraw*
```

Удаление правила:

```sh
sudo ./uninstall.sh
```

## Настройка waybar

В `waybar` модуль:

```json
"custom/mouse": {
    "format": "󰍽 {}",
    "exec": "~/.local/bin/mouse",
    "interval": 10,
    "return-type": "json",
    "class": "{}"
}
```

## Формат вывода

```json
{"text": "85%", "tooltip": "Attack Shark R5\nЗаряд: 85%", "percentage": 85}
```

Поле `class` добавляется только при низком заряде:

- меньше 5 % — `"critical"`;
- до 20 % включительно — `"low"`;
- иначе — класса нет.

## Тесты

```sh
cargo test
```

## Отладка

Если проценты не приходят:

1. Проверь, что правило применилось: `getfacl /dev/hidraw*`.
2. Проверь, что ядро видит мышь с VID `373e`: `cat /sys/class/hidraw/hidraw*/device/uevent`.
3. По проводу донгл может использовать другой протокол: рабочий вариант в `src/main.rs`, экспериментальный с CRC — в `src/new.rs`.
