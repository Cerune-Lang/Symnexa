//! A small statically linked x86-64 ELF image, currently using the LiseOS profile.
use std::collections::HashMap;

use crate::elf::{ElfObject, SHF_ALLOC, SHF_EXECINSTR, SHF_WRITE, SHN_ABS, SHN_UNDEF, SHT_NOBITS, SHT_PROGBITS};

const BASE: u64 = 0xffff_ffff_8000_0000;
const PAGE: u64 = 0x1000;
const MAX_IMAGE: u64 = 64 * 1024 * 1024;
const LIMINE_REQUESTS: [u64; 9] = [
    0xf6b8f4b39de7d1ae, 0xfab91a6940fcb9cf,
    0x785c6ed015d3e316, 0x181e920a7852b9d9,
    0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 3,
    0xadc0e0531bb10d03, 0x9572709f31764c62,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group { Text, Rodata, Data, Bss }
#[derive(Clone, Copy)]
struct Location { group: Group, offset: u64 }
struct Buf {
    bytes: Vec<u8>,
    memory_size: u64,
}
impl Buf {
    fn new() -> Self { Self { bytes: Vec::new(), memory_size: 0 } }
    fn place(&mut self, align: u64, size: u64, data: &[u8]) -> Result<u64, String> {
        let offset = align_up(self.memory_size, align)?;
        let end = offset.checked_add(size).ok_or("section size overflow")?;
        if end > MAX_IMAGE { return Err("image exceeds 64 MiB safety limit".into()); }
        if offset < self.bytes.len() as u64 { return Err("invalid section overlap".into()); }
        if !data.is_empty() {
            self.bytes.resize(offset as usize, 0);
            self.bytes.extend_from_slice(data);
        } else if size != 0 {
            // Only BSS may extend memory without occupying file bytes.
            if !self.bytes.is_empty() { return Err("NOBITS cannot follow file data in a segment".into()); }
        }
        self.memory_size = end;
        Ok(offset)
    }
}
fn align_up(v: u64, align: u64) -> Result<u64, String> {
    if align == 0 || !align.is_power_of_two() { return Err("invalid alignment".into()); }
    let mask = align - 1;
    v.checked_add(mask).map(|v| v & !mask).ok_or_else(|| "address overflow".into())
}
fn put16(dst: &mut [u8], at: usize, v: u16) { dst[at..at+2].copy_from_slice(&v.to_le_bytes()); }
fn put32(dst: &mut [u8], at: usize, v: u32) { dst[at..at+4].copy_from_slice(&v.to_le_bytes()); }
fn put64(dst: &mut [u8], at: usize, v: u64) { dst[at..at+8].copy_from_slice(&v.to_le_bytes()); }
fn add_string(table: &mut Vec<u8>, text: &str) -> Result<u32, String> {
    let at = u32::try_from(table.len()).map_err(|_| "string table too large")?;
    table.extend_from_slice(text.as_bytes());
    table.push(0);
    Ok(at)
}
fn class(flags: u64, name: &str, ty: u32) -> Result<Option<Group>, String> {
    if flags & SHF_ALLOC == 0 { return Ok(None); }
    if ty != SHT_PROGBITS && ty != SHT_NOBITS {
        return Err(format!("unsupported allocatable section: {name} (type {ty})"));
    }
    if name == ".limine_requests" || name.starts_with(".limine_requests.") {
        return Err("Limine requests are emitted by --liseos; input must not duplicate them".into());
    }
    if ty == SHT_NOBITS {
        if flags & SHF_EXECINSTR != 0 { return Err("executable NOBITS section".into()); }
        return Ok(Some(Group::Bss));
    }
    if flags & SHF_EXECINSTR != 0 {
        if flags & SHF_WRITE != 0 { return Err("RWX section is not permitted".into()); }
        Ok(Some(Group::Text))
    } else if flags & SHF_WRITE != 0 {
        Ok(Some(Group::Data))
    } else {
        Ok(Some(Group::Rodata))
    }
}

struct Placement {
    text: Buf,
    rodata: Buf,
    data: Buf,
    bss: Buf,
    request_offset: u64,
    // Section locations in their output groups.
    input_locations: Vec<Vec<Option<Location>>>,
}
impl Placement {
    fn buffer(&mut self, group: Group) -> &mut Buf {
        match group {
            Group::Text => &mut self.text,
            Group::Rodata => &mut self.rodata,
            Group::Data => &mut self.data,
            Group::Bss => &mut self.bss,
        }
    }
}
struct Layout {
    text_off: u64,
    rodata_off: u64,
    requests_off: u64,
    data_off: u64,
    bss_off: u64,
    text_va: u64,
    rodata_va: u64,
    requests_va: u64,
    data_va: u64,
    bss_va: u64,
    file_end: u64,
}
impl Layout {
    fn address(&self, group: Group, offset: u64) -> Result<u64, String> {
        let base = match group {
            Group::Text => self.text_va,
            Group::Rodata => self.rodata_va,
            Group::Data => self.data_va,
            Group::Bss => self.bss_va,
        };
        base.checked_add(offset).ok_or_else(|| "symbol address overflow".into())
    }
    fn file_offset(&self, group: Group, offset: u64) -> Result<u64, String> {
        let base = match group {
            Group::Text => self.text_off,
            Group::Rodata => self.rodata_off,
            Group::Data => self.data_off,
            Group::Bss => self.bss_off,
        };
        base.checked_add(offset).ok_or_else(|| "file offset overflow".into())
    }
}

fn create_layout(p: &Placement) -> Result<Layout, String> {
    let text_off = PAGE;
    let rodata_off = align_up(text_off + p.text.bytes.len() as u64, PAGE)?;
    let requests_off = align_up(rodata_off + p.rodata.bytes.len() as u64, PAGE)?;
    let data_off = align_up(requests_off + LIMINE_REQUESTS.len() as u64 * 8, PAGE)?;
    let bss_off = align_up(data_off + p.data.bytes.len() as u64, PAGE)?;
    let image_end = bss_off.checked_add(p.bss.memory_size).ok_or("image overflow")?;
    if image_end > MAX_IMAGE { return Err("linked image exceeds 64 MiB".into()); }
    Ok(Layout {
        text_off, rodata_off, requests_off, data_off, bss_off,
        text_va: BASE + text_off,
        rodata_va: BASE + rodata_off,
        requests_va: BASE + requests_off,
        data_va: BASE + data_off,
        bss_va: BASE + bss_off,
        file_end: data_off + p.data.bytes.len() as u64,
    })
}

fn section_address(objects: &[ElfObject], p: &Placement, l: &Layout, oi: usize, si: usize) -> Result<u64, String> {
    let location = p.input_locations.get(oi).and_then(|v| v.get(si)).copied().flatten()
        .ok_or_else(|| format!("{}: reference to non-allocatable section {si}", objects[oi].name))?;
    l.address(location.group, location.offset)
}
fn symbol_address(
    objects: &[ElfObject], p: &Placement, l: &Layout, global: &HashMap<String, (usize, usize)>,
    oi: usize, si: usize,
) -> Result<u64, String> {
    let obj = objects.get(oi).ok_or("invalid object")?;
    let sym = obj.symbols.get(si).ok_or("invalid symbol")?;
    if sym.section == SHN_UNDEF {
        if sym.name.is_empty() && si == 0 { return Ok(0); }
        let (owner, id) = global.get(&sym.name)
            .copied().ok_or_else(|| format!("{}: unresolved symbol {}", obj.name, sym.name))?;
        return defined_symbol(objects, p, l, owner, id);
    }
    defined_symbol(objects, p, l, oi, si)
}
fn defined_symbol(objects: &[ElfObject], p: &Placement, l: &Layout, oi: usize, si: usize) -> Result<u64, String> {
    let sym = &objects[oi].symbols[si];
    if sym.section == SHN_ABS { return Ok(sym.value); }
    let sn = sym.section as usize;
    let sec = objects[oi].sections.get(sn).ok_or("invalid symbol section")?;
    if sym.value > sec.size { return Err(format!("{}: symbol {} outside section", objects[oi].name, sym.name)); }
    let address = section_address(objects, p, l, oi, sn)?;
    address.checked_add(sym.value).ok_or_else(|| "symbol address overflow".into())
}
fn relocate(objects: &[ElfObject], p: &mut Placement, l: &Layout, globals: &HashMap<String, (usize, usize)>) -> Result<(), String> {
    for (oi, obj) in objects.iter().enumerate() {
        for rel in &obj.relocations {
            let Some(location) = p.input_locations[oi].get(rel.target_section).copied().flatten() else {
                continue; // Do not apply relocations to non-allocatable debug sections.
            };
            if location.group == Group::Bss { return Err("relocation into BSS is unsupported".into()); }
            let symbol = symbol_address(objects, p, l, globals, oi, rel.symbol)? as i128;
            let place_offset = location.offset.checked_add(rel.offset).ok_or("relocation offset overflow")?;
            let place = l.address(location.group, place_offset)? as i128;
            let value = match rel.kind {
                1 | 10 | 11 => symbol + rel.addend as i128,
                2 | 4 => symbol + rel.addend as i128 - place,
                other => return Err(format!("{}: unsupported x86-64 relocation type {other}", obj.name)),
            };
            let bytes: Vec<u8> = match rel.kind {
                1 => {
                    if !(0..=u64::MAX as i128).contains(&value) {
                        return Err("R_X86_64_64 overflow".into());
                    }
                    (value as u64).to_le_bytes().to_vec()
                }
                10 => {
                    if !(0..=u32::MAX as i128).contains(&value) {
                        return Err("R_X86_64_32 overflow".into());
                    }
                    (value as u32).to_le_bytes().to_vec()
                }
                11 | 2 | 4 => {
                    if !(i32::MIN as i128..=i32::MAX as i128).contains(&value) {
                        return Err(format!("{}: 32-bit relocation overflow", obj.name));
                    }
                    (value as i32).to_le_bytes().to_vec()
                }
                _ => unreachable!(),
            };
            let target = &obj.sections[rel.target_section];
            if rel.offset.checked_add(bytes.len() as u64).is_none_or(|end| end > target.size) {
                return Err(format!("{}: relocation extends beyond section {}", obj.name, target.name));
            }
            let dst = &mut p.buffer(location.group).bytes;
            let start = usize::try_from(place_offset).map_err(|_| "relocation offset overflow")?;
            let end = start.checked_add(bytes.len()).ok_or("relocation offset overflow")?;
            let slot = dst.get_mut(start..end).ok_or("relocation outside file bytes")?;
            slot.copy_from_slice(&bytes);
        }
    }
    Ok(())
}
fn section_header(out: &mut Vec<u8>, name: u32, ty: u32, flags: u64, addr: u64, off: u64, size: u64, link: u32, info: u32, align: u64, entsize: u64) {
    let start = out.len();
    out.resize(start + 64, 0);
    put32(out, start, name);
    put32(out, start+4, ty);
    put64(out, start+8, flags);
    put64(out, start+16, addr);
    put64(out, start+24, off);
    put64(out, start+32, size);
    put32(out, start+40, link);
    put32(out, start+44, info);
    put64(out, start+48, align);
    put64(out, start+56, entsize);
}
fn phdr(out: &mut [u8], index: usize, flags: u32, off: u64, va: u64, filesz: u64, memsz: u64) {
    let p = 64 + index * 56;
    put32(out, p, 1); // PT_LOAD
    put32(out, p+4, flags);
    put64(out, p+8, off);
    put64(out, p+16, va);
    put64(out, p+24, va);
    put64(out, p+32, filesz);
    put64(out, p+40, memsz);
    put64(out, p+48, PAGE);
}
fn write_sym(out: &mut Vec<u8>, name: u32, info: u8, other: u8, section: u16, value: u64, size: u64) {
    let start = out.len();
    out.resize(start+24, 0);
    put32(out, start, name);
    out[start+4] = info;
    out[start+5] = other;
    put16(out, start+6, section);
    put64(out, start+8, value);
    put64(out, start+16, size);
}

pub fn link_liseos(objects: &[ElfObject]) -> Result<Vec<u8>, String> {
    if objects.is_empty() { return Err("no input objects".into()); }
    let mut p = Placement {
        text: Buf::new(), rodata: Buf::new(), data: Buf::new(), bss: Buf::new(),
        request_offset: 0, input_locations: Vec::new(),
    };
    // Merge input sections of each group in input order, retaining their alignments.
    for obj in objects {
        let mut locations = vec![None; obj.sections.len()];
        for (si, sec) in obj.sections.iter().enumerate() {
            if let Some(group) = class(sec.flags, &sec.name, sec.ty)? {
                // Do not allow input NOBITS in other groups.
                if group != Group::Bss && sec.ty == SHT_NOBITS {
                    return Err(format!("{}: unsupported NOBITS section {}", obj.name, sec.name));
                }
                let offset = p.buffer(group).place(sec.align, sec.size, &sec.data)?;
                locations[si] = Some(Location {group, offset});
            }
        }
        p.input_locations.push(locations);
    }
    if p.text.bytes.is_empty() { return Err("no executable code in input".into()); }
    let l = create_layout(&p)?;
    let mut globals = HashMap::<String, (usize, usize)>::new();
    for (oi, obj) in objects.iter().enumerate() {
        for (si, sym) in obj.symbols.iter().enumerate() {
            if sym.section == SHN_UNDEF || sym.name.is_empty() { continue; }
            if sym.section != SHN_ABS && (sym.section as usize >= obj.sections.len()
                || p.input_locations[oi][sym.section as usize].is_none()) {
                if sym.info >> 4 != 0 { return Err(format!("{}: unsupported defined symbol {}", obj.name, sym.name)); }
                continue;
            }
            if sym.info >> 4 == 1 || sym.info >> 4 == 2 { // GLOBAL or WEAK (reject weak duplicate ambiguity)
                if globals.insert(sym.name.clone(), (oi, si)).is_some() {
                    return Err(format!("duplicate global symbol: {}", sym.name));
                }
            }
        }
    }
    // Entry may be local or global, but must resolve unambiguously.
    let entries = objects.iter().enumerate().flat_map(|(oi,obj)| {
        obj.symbols.iter().enumerate().filter_map(move |(si,sym)|
            (sym.name == "main" && sym.section != SHN_UNDEF).then_some((oi,si)))
    }).collect::<Vec<_>>();
    if entries.len() != 1 { return Err("exactly one defined main symbol is required".into()); }
    let entry = defined_symbol(objects, &p, &l, entries[0].0, entries[0].1)?;
    if entry < l.text_va || entry >= l.text_va + p.text.memory_size {
        return Err("main is not in executable section".into());
    }
    relocate(objects, &mut p, &l, &globals)?;

    let mut out = vec![0u8; 64 + 3 * 56];
    out.resize(l.text_off as usize, 0);
    out.extend_from_slice(&p.text.bytes);
    out.resize(l.rodata_off as usize, 0);
    out.extend_from_slice(&p.rodata.bytes);
    out.resize(l.requests_off as usize, 0);
    for value in LIMINE_REQUESTS { out.extend_from_slice(&value.to_le_bytes()); }
    out.resize(l.data_off as usize, 0);
    out.extend_from_slice(&p.data.bytes);

    let mut symbols = vec![0u8; 24]; // mandatory null symbol
    let mut strings = vec![0u8];
    let mut public = globals.iter().map(|(name, owner)| (name.as_str(), *owner)).collect::<Vec<_>>();
    public.sort_by_key(|(name,_)| *name);
    // Include main even if compiler made it local.
    if !globals.contains_key("main") {
        public.push(("main", entries[0]));
    }
    for (name, (oi, si)) in public {
        let symbol = &objects[oi].symbols[si];
        let name_id = add_string(&mut strings, name)?;
        let section = if symbol.section == SHN_ABS { SHN_ABS } else {
            match p.input_locations[oi][symbol.section as usize].unwrap().group {
                Group::Text => 1,
                Group::Rodata => 2,
                Group::Data => 4,
                Group::Bss => 5,
            }
        };
        let value = defined_symbol(objects, &p, &l, oi, si)?;
        write_sym(&mut symbols, name_id, 0x10 | (symbol.info & 0xf), symbol.other, section, value, symbol.size);
    }
    // Section #0 null, then text, rodata, requests, data, bss, symtab, strtab, shstrtab.
    let mut shstrings = vec![0u8];
    let mut shnames = Vec::new();
    for name in [".text",".rodata",".limine_requests",".data",".bss",".symtab",".strtab",".shstrtab"] {
        shnames.push(add_string(&mut shstrings, name)?);
    }
    let symoff = align_up(out.len() as u64, 8)?;
    out.resize(symoff as usize, 0);
    out.extend_from_slice(&symbols);
    let stroff = out.len() as u64;
    out.extend_from_slice(&strings);
    let shstroff = out.len() as u64;
    out.extend_from_slice(&shstrings);
    let shoff = align_up(out.len() as u64, 8)?;
    out.resize(shoff as usize, 0);
    section_header(&mut out, 0,0,0,0,0,0,0,0,0,0);
    section_header(&mut out, shnames[0],1,6,l.text_va,l.text_off,p.text.bytes.len() as u64,0,0,PAGE,0);
    section_header(&mut out, shnames[1],1,2,l.rodata_va,l.rodata_off,p.rodata.bytes.len() as u64,0,0,PAGE,0);
    section_header(&mut out, shnames[2],1,3,l.requests_va,l.requests_off,72,0,0,PAGE,0);
    section_header(&mut out, shnames[3],1,3,l.data_va,l.data_off,p.data.bytes.len() as u64,0,0,PAGE,0);
    section_header(&mut out, shnames[4],8,3,l.bss_va,l.bss_off,p.bss.memory_size,0,0,PAGE,0);
    section_header(&mut out, shnames[5],2,0,0,symoff,symbols.len() as u64,7,1,8,24);
    section_header(&mut out, shnames[6],3,0,0,stroff,strings.len() as u64,0,0,1,0);
    section_header(&mut out, shnames[7],3,0,0,shstroff,shstrings.len() as u64,0,0,1,0);

    // PT_LOAD segments use disjoint pages for RX, R, and RW permissions.
    // Data PT_LOAD includes the Limine descriptors and optional data/BSS.
    let has_rodata = !p.rodata.bytes.is_empty();
    let phnum = if has_rodata { 3 } else { 2 };
    phdr(&mut out, 0, 5, l.text_off, l.text_va, p.text.bytes.len() as u64, p.text.memory_size);
    if has_rodata {
        phdr(&mut out, 1, 4, l.rodata_off, l.rodata_va, p.rodata.bytes.len() as u64, p.rodata.memory_size);
    }
    let data_index = if has_rodata { 2 } else { 1 };
    phdr(&mut out, data_index, 6, l.requests_off, l.requests_va,
         l.file_end - l.requests_off, l.bss_va + p.bss.memory_size - l.requests_va);
    out[0..4].copy_from_slice(b"\x7fELF");
    out[4] = 2; // ELFCLASS64
    out[5] = 1; // little endian
    out[6] = 1; // version
    put16(&mut out,16,2); // ET_EXEC
    put16(&mut out,18,62); // EM_X86_64
    put32(&mut out,20,1);
    put64(&mut out,24,entry);
    put64(&mut out,32,64); // program headers
    put64(&mut out,40,shoff);
    put32(&mut out,48,0);
    put16(&mut out,52,64);
    put16(&mut out,54,56);
    put16(&mut out,56,phnum as u16);
    put16(&mut out,58,64);
    put16(&mut out,60,9);
    put16(&mut out,62,8);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::{Section, Symbol, Relocation};
    fn fixture() -> ElfObject {
        ElfObject {
            name: "unit.o".into(),
            sections: vec![
                Section{name:"".into(),ty:0,flags:0,align:1,size:0,data:vec![]},
                Section{name:".text".into(),ty:1,flags:6,align:16,size:4,data:vec![0x90,0x90,0xeb,0xfe]},
            ],
            symbols: vec![
                Symbol{name:"".into(), section:0, value:0,size:0,info:0,other:0},
                Symbol{name:"main".into(),section:1,value:0,size:4,info:0x12,other:0},
            ],
            relocations: vec![],
        }
    }
    #[test]
    fn produces_limine_ready_elf() {
        let img=link_liseos(&[fixture()]).unwrap();
        assert_eq!(&img[..4],b"\x7fELF");
        assert_eq!(u64::from_le_bytes(img[24..32].try_into().unwrap()),BASE+PAGE);
        assert_eq!(u16::from_le_bytes(img[56..58].try_into().unwrap()),2);
        assert!(img.windows(8).any(|b| b==LIMINE_REQUESTS[0].to_le_bytes()));
        assert!(img.windows(4).any(|b| b==b"main"));
    }
    #[test]
    fn missing_entry_is_rejected() {
        let mut obj=fixture();
        obj.symbols[1].name="not_main".into();
        assert!(link_liseos(&[obj]).unwrap_err().contains("main"));
    }
    #[test]
    fn duplicate_symbols_are_rejected() {
        assert!(link_liseos(&[fixture(),fixture()]).unwrap_err().contains("duplicate"));
    }
    #[test]
    fn unsupported_relocation_is_rejected() {
        let mut obj=fixture();
        obj.relocations.push(Relocation{target_section:1,offset:0,symbol:1,kind:777,addend:0});
        assert!(link_liseos(&[obj]).unwrap_err().contains("unsupported"));
    }
    #[test]
    fn pc_relative_relocation() {
        let mut obj=fixture();
        obj.relocations.push(Relocation{target_section:1,offset:0,symbol:1,kind:2,addend:-4});
        let img=link_liseos(&[obj]).unwrap();
        assert_eq!(&img[0x1000..0x1004],&(-4i32).to_le_bytes());
    }

    #[test]
    fn cross_object_symbol_and_pc32_relocation() {
        let mut caller = fixture();
        caller.symbols.push(Symbol {
            name: "helper".into(), section: SHN_UNDEF, value: 0,
            size: 0, info: 0x10, other: 0,
        });
        caller.relocations.push(Relocation {
            target_section: 1, offset: 0, symbol: 2, kind: 2, addend: -4,
        });
        let mut callee = fixture();
        callee.name = "helper.o".into();
        callee.symbols[1].name = "helper".into();
        let result = link_liseos(&[caller, callee]).unwrap();
        // Input sections have 16-byte alignment: helper is 16 bytes after main.
        assert_eq!(&result[PAGE as usize..PAGE as usize + 4], &(12i32).to_le_bytes());
        assert!(result.windows(7).any(|bytes| bytes == b"helper\0"));
    }

    #[test]
    fn unresolved_symbol_is_rejected() {
        let mut caller = fixture();
        caller.symbols.push(Symbol {
            name: "missing".into(), section: SHN_UNDEF, value: 0,
            size: 0, info: 0x10, other: 0,
        });
        caller.relocations.push(Relocation {
            target_section: 1, offset: 0, symbol: 2, kind: 2, addend: -4,
        });
        assert!(link_liseos(&[caller]).unwrap_err().contains("unresolved symbol missing"));
    }

    #[test]
    fn alignment_and_bss_are_preserved() {
        let mut object = fixture();
        object.sections.push(Section {
            name: ".data".into(), ty: SHT_PROGBITS, flags: SHF_ALLOC | SHF_WRITE,
            align: 8, size: 8, data: vec![1,2,3,4,5,6,7,8],
        });
        object.sections.push(Section {
            name: ".bss".into(), ty: SHT_NOBITS, flags: SHF_ALLOC | SHF_WRITE,
            align: 16, size: 64, data: vec![],
        });
        let result = link_liseos(&[object]).unwrap();
        let phnum = u16::from_le_bytes(result[56..58].try_into().unwrap());
        assert_eq!(phnum, 2);
        let data_program = 64 + 56; // second program header
        let flags = u32::from_le_bytes(result[data_program + 4..data_program + 8].try_into().unwrap());
        let filesz = u64::from_le_bytes(result[data_program + 32..data_program + 40].try_into().unwrap());
        let memsz = u64::from_le_bytes(result[data_program + 40..data_program + 48].try_into().unwrap());
        assert_eq!(flags, 6);
        assert!(memsz > filesz);
    }
}
