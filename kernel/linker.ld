/* tuiOS-Prime: kernel estático no higher-half (Limine 8.x exige PHDRs altos).
   Cada PT_LOAD em página exclusiva (ALIGN): Limine rejeita permissões
   diferentes compartilhando a mesma página. */
ENTRY(_start)

PHDRS {
    text PT_LOAD;
    rodata PT_LOAD;
    data PT_LOAD;
}

SECTIONS {
    . = 0xFFFFFFFF80000000 + SIZEOF_HEADERS;

    .text : {
        *(.text .text.*)
    } :text

    . = ALIGN(0x1000);

    .rodata : {
        *(.rodata .rodata.*)
        KEEP(*(.limine_requests))
    } :rodata

    . = ALIGN(0x1000);

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } :data

    . = ALIGN(0x1000);

    .bss : {
        *(COMMON)
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    } :data

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
    }
}
