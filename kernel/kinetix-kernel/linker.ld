ENTRY(_start)

SECTIONS {
    /* Kernel base at 2 MiB (standard physical load address for x86_64 kernels) */
    . = 0x00200000;

    .text ALIGN(4K) : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : {
        *(COMMON)
        *(.bss .bss.*)
    }
}
