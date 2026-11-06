//! ELF64 Header & Program Header Parser for Kinetix OS Native Applications.
//!
//! Provides zero-copy parsing and validation of 64-bit ELF executable binaries
//! conforming to the System V AMD64 ABI.

use core::slice;

pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
pub const ELF_CLASS_64: u8 = 2;
pub const ELF_DATA_2LSB: u8 = 1;
pub const EM_X86_64: u16 = 0x3E;

pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;

pub const PT_LOAD: u32 = 1;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
#[allow(dead_code)]
pub const PF_R: u32 = 4;

/// Standard 64-bit ELF File Header.
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

/// Standard 64-bit ELF Program Header describing memory segments.
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

impl Elf64ProgramHeader {
    pub fn is_load(&self) -> bool {
        self.p_type == PT_LOAD
    }

    pub fn is_executable(&self) -> bool {
        (self.p_flags & PF_X) != 0
    }

    pub fn is_writable(&self) -> bool {
        (self.p_flags & PF_W) != 0
    }

    #[allow(dead_code)]
    pub fn is_readable(&self) -> bool {
        (self.p_flags & PF_R) != 0
    }
}

/// Parsing errors encountered while decoding an ELF64 binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    BufferTooSmall,
    InvalidMagic,
    Not64Bit,
    NotLittleEndian,
    NotX86_64,
    UnsupportedType,
    InvalidProgramHeaders,
}

/// Validated zero-copy wrapper over an in-memory ELF64 binary.
pub struct ElfBinary<'a> {
    buffer: &'a [u8],
    header: &'a Elf64Header,
    program_headers: &'a [Elf64ProgramHeader],
}

impl<'a> ElfBinary<'a> {
    /// Parses and strictly validates an ELF64 binary from a byte slice.
    pub fn parse(buffer: &'a [u8]) -> Result<Self, ElfError> {
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
        if header.e_type != ET_EXEC && header.e_type != ET_DYN {
            return Err(ElfError::UnsupportedType);
        }

        let ph_offset = header.e_phoff as usize;
        let ph_count = header.e_phnum as usize;
        let ph_size = header.e_phentsize as usize;

        if ph_offset + ph_count * ph_size > buffer.len() {
            return Err(ElfError::InvalidProgramHeaders);
        }

        let program_headers = unsafe {
            slice::from_raw_parts(
                buffer.as_ptr().add(ph_offset) as *const Elf64ProgramHeader,
                ph_count,
            )
        };

        Ok(Self {
            buffer,
            header,
            program_headers,
        })
    }

    /// Virtual memory entry point address where execution begins.
    pub fn entry_point(&self) -> u64 {
        self.header.e_entry
    }

    /// Program headers slice.
    #[allow(dead_code)]
    pub fn program_headers(&self) -> &'a [Elf64ProgramHeader] {
        self.program_headers
    }

    /// Iterator over loadable program segments (`PT_LOAD`).
    pub fn load_segments(&self) -> impl Iterator<Item = &'a Elf64ProgramHeader> {
        self.program_headers.iter().filter(|ph| ph.is_load())
    }

    /// Returns the raw file slice corresponding to a program segment.
    pub fn segment_data(&self, ph: &Elf64ProgramHeader) -> Option<&'a [u8]> {
        let start = ph.p_offset as usize;
        let end = start + ph.p_filesz as usize;
        if end <= self.buffer.len() {
            Some(&self.buffer[start..end])
        } else {
            None
        }
    }
}
