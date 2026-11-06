#![no_std]

/// Magic signature value placed in `BootInfo` to verify integrity ("KINETIX!" in ASCII).
pub const KINETIX_BOOT_MAGIC: u64 = 0x4B49_4E45_5449_5821;

/// Protocol version number.
pub const KINETIX_BOOT_VERSION: u32 = 1;

/// Maximum number of physical memory map regions passed to the kernel.
pub const MAX_MEMORY_REGIONS: usize = 256;

/// Maximum number of boot modules (preloaded application binaries) supported.
pub const MAX_MODULES: usize = 16;

/// Primary boot information structure passed in register `RDI` from Kinetix Bootloader to Kinetix Kernel.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// Integrity magic: must equal `KINETIX_BOOT_MAGIC`.
    pub magic: u64,
    /// Protocol version: must equal `KINETIX_BOOT_VERSION`.
    pub version: u32,
    /// Reserved alignment padding.
    pub _reserved: u32,
    /// Physical memory map of the host system.
    pub memory_map: MemoryMap,
    /// Linear physical framebuffer for graphical / console output.
    pub framebuffer: FramebufferInfo,
    /// Array of preloaded modules (e.g., initial user-space applications).
    pub modules: ModuleList,
    /// Physical address of the ACPI Root System Description Pointer (RSDP), if found.
    pub acpi_rsdp: u64,
}

impl BootInfo {
    /// Validates the magic signature and protocol version.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.magic == KINETIX_BOOT_MAGIC && self.version == KINETIX_BOOT_VERSION
    }
}

/// System memory map detailing usable, reserved, and firmware regions.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    /// Array of memory region descriptors.
    pub regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    /// Actual number of valid entries in `regions`.
    pub region_count: usize,
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self {
            regions: [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS],
            region_count: 0,
        }
    }
}

/// A contiguous physical memory region.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Starting physical base address (4 KiB page aligned).
    pub start: u64,
    /// Total size of this region in bytes.
    pub length: u64,
    /// Classification of this memory region.
    pub region_type: MemoryType,
    /// Reserved padding.
    pub _padding: u32,
}

impl MemoryRegion {
    pub const EMPTY: Self = Self {
        start: 0,
        length: 0,
        region_type: MemoryType::Reserved,
        _padding: 0,
    };

    #[inline]
    pub fn end(&self) -> u64 {
        self.start + self.length
    }

    #[inline]
    pub fn is_usable(&self) -> bool {
        self.region_type == MemoryType::Usable
    }
}

/// Memory region classification.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Usable general-purpose physical RAM available to the frame allocator.
    Usable = 1,
    /// Reserved by firmware or hardware; must not be modified.
    Reserved = 2,
    /// Memory occupied by the loaded Kinetix Kernel image.
    KernelCode = 3,
    /// Memory occupied by boot modules (staged applications).
    BootloaderModule = 4,
    /// ACPI reclaimable tables (can be reused after reading ACPI).
    AcpiReclaimable = 5,
    /// ACPI Non-Volatile Sleep memory.
    AcpiNvs = 6,
    /// Memory-mapped I/O region.
    Mmio = 7,
    /// Framebuffer video memory.
    Framebuffer = 8,
}

/// Framebuffer metadata for linear direct-pixel rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Physical base address of the video memory.
    pub physical_address: u64,
    /// Total size of the framebuffer buffer in bytes.
    pub size: usize,
    /// Horizontal resolution in pixels.
    pub width: u32,
    /// Vertical resolution in pixels.
    pub height: u32,
    /// Stride: number of pixels per scanline (may include padding).
    pub stride: u32,
    /// Pixel encoding format.
    pub pixel_format: PixelFormat,
}

impl Default for FramebufferInfo {
    fn default() -> Self {
        Self {
            physical_address: 0,
            size: 0,
            width: 0,
            height: 0,
            stride: 0,
            pixel_format: PixelFormat::Unknown,
        }
    }
}

/// Pixel color format for the framebuffer.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Unknown = 0,
    /// Red: Green: Blue: Reserved (8-bits each).
    Rgb = 1,
    /// Blue: Green: Red: Reserved (8-bits each).
    Bgr = 2,
}

/// List of preloaded native binary modules.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModuleList {
    pub modules: [ModuleDescriptor; MAX_MODULES],
    pub count: usize,
}

impl Default for ModuleList {
    fn default() -> Self {
        Self {
            modules: [ModuleDescriptor::EMPTY; MAX_MODULES],
            count: 0,
        }
    }
}

/// Descriptor for a staged native application binary loaded into memory.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModuleDescriptor {
    /// Physical address where the binary was staged by the bootloader.
    pub physical_address: u64,
    /// Length of the executable binary in bytes.
    pub size: usize,
    /// UTF-8 null-terminated module identifier or filename.
    pub name: [u8; 32],
}

impl ModuleDescriptor {
    pub const EMPTY: Self = Self {
        physical_address: 0,
        size: 0,
        name: [0u8; 32],
    };
}
