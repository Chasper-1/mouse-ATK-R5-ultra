#!/bin/sh
#
# Ставит udev-правило для ATTACK SHARK R5 Ultra.
# После установки программа заряда и Waybar работают без sudo.
#
#   ./install.sh

set -eu

RULE_DEST="/etc/udev/rules.d/70-attack-shark.rules"

command -v udevadm >/dev/null 2>&1 || {
    echo "Ошибка: udevadm не найден — на системе нет udev/systemd" >&2
    exit 1
}

if [ "$(id -u)" -eq 0 ]; then
    run_root() { "$@"; }
elif command -v sudo >/dev/null 2>&1; then
    run_root() { sudo -- "$@"; }
elif command -v pkexec >/dev/null 2>&1; then
    run_root() { pkexec -- "$@"; }
else
    echo "Ошибка: нужны права root (sudo/pkexec)" >&2
    exit 1
fi

run_root tee "$RULE_DEST" >/dev/null <<'EOF'
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="373e", TAG+="uaccess"
EOF

run_root udevadm control --reload-rules
run_root udevadm trigger --action=change --subsystem-match=hidraw
run_root udevadm settle

echo "Правило установлено: $RULE_DEST"
