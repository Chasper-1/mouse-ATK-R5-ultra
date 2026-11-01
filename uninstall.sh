#!/bin/sh
#
# Удаляет udev-правило для ATTACK SHARK R5 Ultra.
#
#   ./uninstall.sh

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

if [ -f "$RULE_DEST" ]; then
    run_root rm -f -- "$RULE_DEST"
else
    echo "Правило отсутствует: $RULE_DEST"
fi

run_root udevadm control --reload-rules
run_root udevadm trigger --action=change --subsystem-match=hidraw
run_root udevadm settle

echo "Готово."
