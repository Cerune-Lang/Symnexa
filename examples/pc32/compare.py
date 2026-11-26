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


def compare(args, root, case, defines, main_offset, field, displacement):
    obj = root / "input.o"
    subprocess.run([args.clang, "--target=x86_64-unknown-linux-gnu", *defines,
                    "-c", HERE / "input.S", "-o", obj], check=True)
    elf = root / "rust.elf"
    rust = subprocess.run([args.symnexa, "link", "--liseos", "-o", elf, obj],
                          capture_output=True, text=True)
    if displacement is None:
        if rust.returncode == 0 or "32-bit relocation overflow" not in rust.stderr or elf.exists():
            raise RuntimeError(f"{case}: expected Rust overflow, got {rust}")
    else:
        rust.check_returncode()
        entry = struct.unpack_from("<Q", elf.read_bytes(), 24)[0]
        if entry != TEXT_BASE + main_offset:
            raise RuntimeError(f"{case}: unexpected Rust entry {entry:#x}")
        reference = root / "rust.text"
        subprocess.run([args.objcopy, "-O", "binary", "--only-section=.text", elf, reference], check=True)
        text = reference.read_bytes()
        if text[field:field + 4] != struct.pack("<i", displacement):
            raise RuntimeError(f"{case}: unexpected Rust displacement in {text.hex()}")

    for command, *options in ROUTES:
        files = root / "-".join([command, *options])
        files.mkdir()
        shutil.copyfile(obj, files / "input.o")
        result = subprocess.run([args.cerune, command, HERE / "apply.ceru", *options,
                                 "--file-root", files], check=True, capture_output=True, text=True)
        output = files / "text.bin"
        if displacement is None:
            if result.stdout.splitlines() != ["PC32 overflow"] or output.exists():
                raise RuntimeError(f"{case}/{command}: expected rejection, got {result.stdout!r}")
        elif result.stdout.splitlines() != [str(entry), str(displacement)] or output.read_bytes() != text:
            raise RuntimeError(f"{case}/{command}: entry, displacement or text differs: {result.stdout!r}")
    if displacement is None:
        print(f"{case}: Rust and all four Cerune routes rejected the relocation")
    else:
        print(f"{case}: entry={entry:#x} displacement={displacement} text={text.hex(' ')}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cerune", required=True)
    parser.add_argument("--symnexa", required=True)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--objcopy", default="objcopy")
    args = parser.parse_args()
    for case, defines, main_offset, field, displacement in [
        ("forward", [], 1, 2, 2),
        ("backward", ["-DBACKWARD"], 2, 3, -7),
        ("overflow", ["-DOVERFLOW"], 1, 2, None),
        ("underflow", ["-DBACKWARD", "-DUNDERFLOW"], 2, 3, None),
    ]:
        with tempfile.TemporaryDirectory(prefix=f"symnexa-{case}-") as tmp:
            compare(args, Path(tmp), case, defines, main_offset, field, displacement)


if __name__ == "__main__":
    main()
