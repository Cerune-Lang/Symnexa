"""Compare one ELF64 PC32 relocation against the Rust LiseOS linker."""
import argparse
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEXT_BASE = 0xFFFFFFFF80001000
ROUTES = (("run",), ("run-mir",), ("run-mir", "--ssa"), ("run-vm",))


def assemble(args, source, defines, obj):
    subprocess.run([args.clang, "--target=x86_64-unknown-linux-gnu", *defines,
                    "-c", HERE / source, "-o", obj], check=True)


def rx_segment(image):
    ident, kind, machine, version, entry, phoff, _, _, ehsize, stride, count, *_ = struct.unpack_from(
        "<16sHHIQQQIHHHHHH", image)
    if (ident[:7], kind, machine, version, ehsize, stride) != (b"\x7fELF\x02\x01\x01", 2, 62, 1, 64, 56):
        raise ValueError("expected a little-endian x86-64 ELF64 executable")
    segments = [struct.unpack_from("<II6Q", image, phoff + i * stride) for i in range(count)]
    (rx,) = [segment for segment in segments if segment[:2] == (1, 5)]
    _, _, offset, address, _, size, _, _ = rx
    if offset + size > len(image) or not address <= entry < address + size:
        raise ValueError("entry or executable segment outside image")
    return entry, rx, image[offset:offset + size]


def compare(args, root, case, objects, program, symbol, main_offset, field, displacement, errors=None, image_error=None):
    elf = root / "rust.elf"
    rust = subprocess.run([args.symnexa, "link", "--liseos", "-o", elf, *objects],
                          capture_output=True, text=True)
    if errors:
        if rust.returncode == 0 or errors[0] not in rust.stderr or elf.exists():
            raise RuntimeError(f"{case}: expected {errors[0]!r}, got {rust}")
    else:
        rust.check_returncode()
        entry = struct.unpack_from("<Q", elf.read_bytes(), 24)[0]
        if entry != TEXT_BASE + main_offset:
            raise RuntimeError(f"{case}: unexpected Rust entry {entry:#x}")
        # All successful fixtures use an addend of -4. Recover S from P and the displacement
        # and compare it with the symbol address recorded by the Rust linker.
        symbols = subprocess.check_output([args.nm, "-P", elf], text=True)
        addresses = [int(row[2], 16) for line in symbols.splitlines()
                   if (row := line.split()) and row[0] == symbol]
        if addresses != [TEXT_BASE + field + displacement + 4]:
            raise RuntimeError(f"{case}: unexpected Rust symbol address {addresses}")
        reference = root / "rust.text"
        subprocess.run([args.objcopy, "-O", "binary", "--only-section=.text", elf, reference], check=True)
        text = reference.read_bytes()
        if text[field:field + 4] != struct.pack("<i", displacement):
            raise RuntimeError(f"{case}: unexpected Rust displacement in {text.hex()}")

    for command, *options in ROUTES:
        files = root / "-".join([command, *options])
        files.mkdir()
        for obj in objects:
            shutil.copyfile(obj, files / obj.name)
        result = subprocess.run([args.cerune, command, HERE / program, *options,
                                 "--file-root", files], check=True, capture_output=True, text=True)
        output = files / "text.bin"
        if errors:
            if result.stdout.splitlines() != errors[1].splitlines() or output.exists():
                raise RuntimeError(f"{case}/{command}: expected rejection, got {result.stdout!r}")
        elif result.stdout.splitlines() != [str(entry), str(displacement)] or output.read_bytes() != text:
            raise RuntimeError(f"{case}/{command}: entry, displacement or text differs: {result.stdout!r}")
        if program == "pair.ceru":
            result = subprocess.run([args.cerune, command, HERE / "pair_exec.ceru", *options,
                                     "--file-root", files], check=True, capture_output=True, text=True)
            image = files / "pair.elf"
            expected_error = errors[1] if errors else image_error
            if expected_error:
                if result.stdout.splitlines() != expected_error.splitlines() or image.exists():
                    raise RuntimeError(f"{case}/{command}: expected image rejection, got {result.stdout!r}")
            else:
                raw = image.read_bytes()
                actual = rx_segment(raw)
                if actual != rx_segment(elf.read_bytes()) or result.stdout.splitlines() != [str(entry), str(displacement)]:
                    raise RuntimeError(f"{case}/{command}: ELF entry, RX header, payload or printed values differ")
                if struct.unpack_from("<H", raw, 56)[0] != 1 or len(raw) != 4096 + len(text):
                    raise RuntimeError(f"{case}/{command}: expected a single code segment")
    if errors:
        print(f"{case}: Rust and all four Cerune routes rejected the relocation")
    else:
        patch = text[field:field + 4].hex(" ")
        print(f"{case}: entry={entry:#x} {symbol}={addresses[0]:#x} displacement={displacement} "
              f"text_size={len(text)} patch={patch}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cerune", required=True)
    parser.add_argument("--symnexa", required=True)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--objcopy", default="objcopy")
    parser.add_argument("--nm", default="nm")
    args = parser.parse_args()
    for case, defines, main_offset, field, displacement in [
        ("forward", [], 1, 2, 2),
        ("backward", ["-DBACKWARD"], 2, 3, -7),
        ("rodata", ["-DRODATA"], 1, 2, 4109),
        ("rodata-page-end", ["-DRODATA", "-DTEXT_SIZE=4096"], 1, 2, 4109),
        ("rodata-next-page", ["-DRODATA", "-DTEXT_SIZE=4097"], 1, 2, 8205),
        ("rodata-overflow", ["-DRODATA", "-DPLACEMENT_OVERFLOW"], 1, 2, None),
        ("overflow", ["-DOVERFLOW"], 1, 2, None),
        ("underflow", ["-DBACKWARD", "-DUNDERFLOW"], 2, 3, None),
    ]:
        with tempfile.TemporaryDirectory(prefix=f"symnexa-{case}-") as tmp:
            root = Path(tmp)
            obj = root / "input.o"
            assemble(args, "input.S", defines, obj)
            errors = ("32-bit relocation overflow", "PC32 overflow") if displacement is None else None
            compare(args, root, case, [obj], "apply.ceru", "anchor", main_offset, field, displacement, errors)

    missing = ("unresolved symbol answer", f"unresolved symbol\n{list(b'answer')}")
    duplicate = ("duplicate global symbol: shared", f"duplicate global symbol\n{list(b'shared')}")
    for case, reverse, reference, definition, flags, errors in [
        ("external", False, "answer", "answer", [], None),
        ("reversed", True, "other_symbol", "other_symbol", [], None),
        ("extra-data", False, "answer", "answer", ["-DEXTRA_DATA"], None),
        ("weak", False, "answer", "answer", ["-DWEAK"], None),
        ("missing", False, "answer", "answer_extra", [], missing),
        ("local", False, "answer", "answer", ["-DLOCAL"], missing),
        ("duplicate", False, "answer", "answer", ["-DDUPLICATE"], duplicate),
    ]:
        with tempfile.TemporaryDirectory(prefix=f"symnexa-{case}-") as tmp:
            root = Path(tmp)
            first, second = root / "first.o", root / "second.o"
            caller, provider = (second, first) if reverse else (first, second)
            assemble(args, "caller.S", [f"-DREF={reference}", *flags], caller)
            assemble(args, "provider.S", [f"-DDEF={definition}", *flags], provider)
            main_offset, field, displacement = (9, 10, -11) if reverse else (1, 2, 13)
            compare(args, root, case, [first, second], "pair.ceru", reference,
                    main_offset, field, displacement, errors,
                    image_error="expected only text in the load image" if case == "extra-data" else None)


if __name__ == "__main__":
    main()
