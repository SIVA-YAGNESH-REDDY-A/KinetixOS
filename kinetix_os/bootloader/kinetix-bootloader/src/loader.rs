//! In-Memory ELF64 Executable Loader for Kinetix Bootloader

use core::slice;

pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
pub const ELF_CLASS_64: u8 = 2;
pub const ELF_DATA_2LSB: u8 = 1;
pub const EM_X86_64: u16 = 0x3E;
pub const PT_LOAD: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[derive(Debug)]
pub enum ElfError {
    BufferTooSmall,
    InvalidMagic,
    Not64Bit,
    NotLittleEndian,
    NotX86_64,
    InvalidProgramHeaders,
}

/// Parses an ELF64 binary buffer, validates its headers, and returns the entry point.
pub fn parse_and_validate_elf(buffer: &[u8]) -> Result<(u64, &[Elf64ProgramHeader]), ElfError> {
    if buffer.len() < core::mem::size_of::<Elf64Header>() {
        return Err(ElfError::BufferTooSmall);
    }

    let header = unsafe { &*(buffer.as_ptr() as *const Elf64Header) };

    if header.e_ident[0..4] != ELF_MAGIC {
        return Err(ElfError::InvalidMagic);
    }
    if header.e_ident[4] != ELF_CLASS_64 {
        return Err(ElfError::Not64Bit);
    }
    if header.e_ident[5] != ELF_DATA_2LSB {
        return Err(ElfError::NotLittleEndian);
    }
    if header.e_machine != EM_X86_64 {
        return Err(ElfError::NotX86_64);
    }

    let ph_offset = header.e_phoff as usize;
    let ph_count = header.e_phnum as usize;
    let ph_size = header.e_phentsize as usize;

    if ph_offset + ph_count * ph_size > buffer.len() {
        return Err(ElfError::InvalidProgramHeaders);
    }

    let ph_slice = unsafe {
        slice::from_raw_parts(
            buffer.as_ptr().add(ph_offset) as *const Elf64ProgramHeader,
            ph_count,
        )
    };

    Ok((header.e_entry, ph_slice))
}
