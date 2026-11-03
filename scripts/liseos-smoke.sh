#!/usr/bin/env bash
# Build a LiseOS ISO from an already linked Symnexa ELF; do not rebuild with ld.lld.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../LiseOS" && pwd)"
BUILD="$ROOT/build"
LIMINE_COMMIT="5be26a73d7b7b4d4477d18be94e1d16e615adf56"
LIMINE_DIR="$ROOT/deps/limine"
test -f "$BUILD/liseos.elf"

if [[ ! -d "$LIMINE_DIR/.git" ]]; then
  mkdir -p "$ROOT/deps"
  git clone --depth 1 --branch v11.4.1-binary https://github.com/Limine-Bootloader/Limine.git "$LIMINE_DIR"
fi
test "$(git -C "$LIMINE_DIR" rev-parse HEAD)" = "$LIMINE_COMMIT"

if [[ ! -x "$LIMINE_DIR/limine" ]]; then
  make -C "$LIMINE_DIR"
fi
mkdir -p "$BUILD/iso_root/boot/limine" "$BUILD/iso_root/EFI/BOOT"
cp "$BUILD/liseos.elf" "$BUILD/iso_root/boot/liseos.elf"
cp "$ROOT/limine.conf" "$BUILD/iso_root/boot/limine/limine.conf"
cp "$LIMINE_DIR/limine-bios-cd.bin" "$LIMINE_DIR/limine-uefi-cd.bin" "$LIMINE_DIR/limine-bios.sys" "$BUILD/iso_root/boot/"
cp "$LIMINE_DIR/BOOTX64.EFI" "$BUILD/iso_root/EFI/BOOT/BOOTX64.EFI"
xorriso -as mkisofs -R -r -J \
  -b boot/limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot boot/limine-uefi-cd.bin -efi-boot-part --efi-boot-image \
  --protective-msdos-label "$BUILD/iso_root" -o "$BUILD/liseos.iso"
"$LIMINE_DIR/limine" bios-install "$BUILD/liseos.iso"
bash "$ROOT/scripts/smoke-qemu.sh"
