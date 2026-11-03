//! Minimal, checked ELF64/x86-64 ET_REL reader. No external dependencies.
pub const SHT_PROGBITS: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;
pub const SHT_STRTAB: u32 = 3;
pub const SHT_RELA: u32 = 4;
pub const SHT_NOBITS: u32 = 8;
pub const SHT_REL: u32 = 9;
pub const SHF_WRITE: u64 = 1;
pub const SHF_ALLOC: u64 = 2;
pub const SHF_EXECINSTR: u64 = 4;
pub const SHN_UNDEF: u16 = 0;
pub const SHN_ABS: u16 = 0xfff1;

const MAX_SECTION_SIZE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub ty: u32,
    pub flags: u64,
    pub align: u64,
    pub size: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub section: u16,
    pub value: u64,
    pub size: u64,
    pub info: u8,
    pub other: u8,
}

#[derive(Clone, Debug)]
pub struct Relocation {
    pub target_section: usize,
    pub offset: u64,
    pub symbol: usize,
    pub kind: u32,
    pub addend: i64,
}

#[derive(Clone, Debug)]
pub struct ElfObject {
    pub name: String,
    pub sections: Vec<Section>,
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<Relocation>,
}

#[derive(Clone)]
struct Header {
    name: u32,
    ty: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: usize,
    info: usize,
    align: u64,
    entsize: u64,
}

pub fn range(data: &[u8], offset: u64, len: u64) -> Result<&[u8], String> {
    let start = usize::try_from(offset).map_err(|_| "file offset overflow")?;
    let size = usize::try_from(len).map_err(|_| "file length overflow")?;
    let end = start.checked_add(size).ok_or("file range overflow")?;
    data.get(start..end).ok_or_else(|| "truncated ELF input".into())
}
pub fn u16_at(data: &[u8], offset: u64) -> Result<u16, String> {
    Ok(u16::from_le_bytes(range(data, offset, 2)?.try_into().unwrap()))
}
pub fn u32_at(data: &[u8], offset: u64) -> Result<u32, String> {
    Ok(u32::from_le_bytes(range(data, offset, 4)?.try_into().unwrap()))
}
pub fn u64_at(data: &[u8], offset: u64) -> Result<u64, String> {
    Ok(u64::from_le_bytes(range(data, offset, 8)?.try_into().unwrap()))
}
fn cstring(bytes: &[u8], offset: u32) -> Result<String, String> {
    let tail = bytes.get(offset as usize..).ok_or("string offset out of range")?;
    let end = tail.iter().position(|x| *x == 0).ok_or("unterminated string")?;
    std::str::from_utf8(&tail[..end])
        .map(str::to_string)
        .map_err(|_| "invalid UTF-8 in ELF string table".into())
}

impl ElfObject {
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Self, String> {
        Self::parse_inner(name, bytes).map_err(|e| format!("{name}: {e}"))
    }

    fn parse_inner(name: &str, bytes: &[u8]) -> Result<Self, String> {
        if range(bytes, 0, 16)?[0..4] != *b"\x7fELF"
            || bytes[4] != 2
            || bytes[5] != 1
            || bytes[6] != 1
        {
            return Err("expected little-endian ELF64".into());
        }
        if u16_at(bytes, 16)? != 1 || u16_at(bytes, 18)? != 62 || u32_at(bytes, 20)? != 1 {
            return Err("expected x86-64 ET_REL object".into());
        }
        let shoff = u64_at(bytes, 40)?;
        let shentsize = u16_at(bytes, 58)? as u64;
        let shnum = u16_at(bytes, 60)? as usize;
        let shstrndx = u16_at(bytes, 62)? as usize;
        if shentsize < 64 || shnum == 0 || shstrndx == 0 || shstrndx >= shnum {
            return Err("unsupported section header table".into());
        }
        let table_size = shentsize.checked_mul(shnum as u64).ok_or("section table overflow")?;
        range(bytes, shoff, table_size)?;
        let mut headers = Vec::with_capacity(shnum);
        for i in 0..shnum {
            let p = shoff + i as u64 * shentsize;
            headers.push(Header {
                name: u32_at(bytes, p)?,
                ty: u32_at(bytes, p + 4)?,
                flags: u64_at(bytes, p + 8)?,
                offset: u64_at(bytes, p + 24)?,
                size: u64_at(bytes, p + 32)?,
                link: u32_at(bytes, p + 40)? as usize,
                info: u32_at(bytes, p + 44)? as usize,
                align: u64_at(bytes, p + 48)?,
                entsize: u64_at(bytes, p + 56)?,
            });
        }
        if headers[shstrndx].ty != SHT_STRTAB {
            return Err("invalid section name table".into());
        }
        let names = range(bytes, headers[shstrndx].offset, headers[shstrndx].size)?;
        let mut sections = Vec::with_capacity(shnum);
        for h in &headers {
            if h.size > MAX_SECTION_SIZE {
                return Err("section exceeds 64 MiB limit".into());
            }
            if h.align > 1 && !h.align.is_power_of_two() {
                return Err("non-power-of-two section alignment".into());
            }
            let data = if h.ty == SHT_NOBITS {
                Vec::new()
            } else {
                range(bytes, h.offset, h.size)?.to_vec()
            };
            sections.push(Section {
                name: cstring(names, h.name)?,
                ty: h.ty,
                flags: h.flags,
                align: h.align.max(1),
                size: h.size,
                data,
            });
        }
        let symtabs: Vec<usize> = headers.iter().enumerate()
            .filter_map(|(i, h)| (h.ty == SHT_SYMTAB).then_some(i))
            .collect();
        if symtabs.len() != 1 {
            return Err("exactly one SYMTAB is required".into());
        }
        let tab_index = symtabs[0];
        let tab = &headers[tab_index];
        if tab.link >= shnum || headers[tab.link].ty != SHT_STRTAB
            || tab.entsize < 24 || tab.size % tab.entsize != 0
        {
            return Err("invalid SYMTAB metadata".into());
        }
        let strings = &sections[tab.link].data;
        let mut symbols = Vec::new();
        for i in 0..tab.size / tab.entsize {
            let p = i * tab.entsize;
            let b = &sections[tab_index].data;
            symbols.push(Symbol {
                name: cstring(strings, u32_at(b, p)?)?,
                info: range(b, p + 4, 1)?[0],
                other: range(b, p + 5, 1)?[0],
                section: u16_at(b, p + 6)?,
                value: u64_at(b, p + 8)?,
                size: u64_at(b, p + 16)?,
            });
        }
        let mut relocations = Vec::new();
        for h in &headers {
            if h.ty != SHT_RELA && h.ty != SHT_REL {
                continue;
            }
            if h.info >= shnum || h.link != tab_index {
                return Err("relocation section has invalid target or symbol table".into());
            }
            if h.ty == SHT_REL {
                if sections[h.info].flags & SHF_ALLOC != 0 {
                    return Err("SHT_REL relocations are not supported".into());
                }
                continue;
            }
            if h.entsize < 24 || h.size % h.entsize != 0 {
                return Err("invalid RELA entries".into());
            }
            for i in 0..h.size / h.entsize {
                let p = i * h.entsize;
                let b = range(bytes, h.offset, h.size)?;
                let info = u64_at(b, p + 8)?;
                let sym = (info >> 32) as usize;
                if sym >= symbols.len() {
                    return Err("relocation references invalid symbol".into());
                }
                relocations.push(Relocation {
                    target_section: h.info,
                    offset: u64_at(b, p)?,
                    symbol: sym,
                    kind: info as u32,
                    addend: u64_at(b, p + 16)? as i64,
                });
            }
        }
        Ok(Self { name: name.into(), sections, symbols, relocations })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_wrong_format() {
        assert!(ElfObject::parse("broken", b"hello").is_err());
        let mut header = vec![0; 64];
        header[0..4].copy_from_slice(b"\x7fELF");
        header[4] = 2;
        header[5] = 1;
        header[6] = 1;
        assert!(ElfObject::parse("empty", &header).is_err());
    }
}
