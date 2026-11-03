//! Symnexa: host-side linker. This executable is not part of the LiseOS kernel.
mod elf;
mod link;

use std::{env, fs, path::PathBuf, process};

fn usage() -> &'static str {
    "usage: symnexa link --liseos -o OUTPUT.elf INPUT.o [INPUT2.o ...]"
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("link")) {
        return Err(usage().into());
    }
    let mut output: Option<PathBuf> = None;
    let mut inputs = Vec::new();
    let mut liseos = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--liseos") => liseos = true,
            Some("-o") => {
                if output.is_some() {
                    return Err("duplicate -o".into());
                }
                output = Some(PathBuf::from(args.next().ok_or("-o requires a path")?));
            }
            Some("-h" | "--help") => {
                println!("{}", usage());
                return Ok(());
            }
            Some(s) if s.starts_with('-') => return Err(format!("unknown option: {s}")),
            _ => inputs.push(PathBuf::from(arg)),
        }
    }
    if !liseos {
        return Err("only the --liseos linker profile is supported".into());
    }
    let output = output.ok_or_else(|| format!("missing -o; {}", usage()))?;
    if inputs.is_empty() {
        return Err("at least one input object is required".into());
    }
    let objects = inputs
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            elf::ElfObject::parse(&path.display().to_string(), &bytes)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let image = link::link_liseos(&objects)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(&output, &image).map_err(|e| format!("{}: {e}", output.display()))?;
    println!("Symnexa: {} ({} bytes)", output.display(), image.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("symnexa: {error}");
        process::exit(1);
    }
}
