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


def compare(args, root, case, objects, program, symbol, main_offset, field, displacement, errors=None):
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
                    main_offset, field, displacement, errors)


if __name__ == "__main__":
    main()
