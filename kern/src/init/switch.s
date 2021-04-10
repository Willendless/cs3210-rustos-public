.global switch_threads
switch_threads:
    // Kernel threads switch
    // switch_threads (Context *prev, Context *next)
    str x19, [x0], #8
    str x20, [x0], #8
    str x21, [x0], #8
    str x22, [x0], #8
    str x23, [x0], #8
    str x24, [x0], #8
    str x25, [x0], #8
    str x26, [x0], #8
    str x27, [x0], #8
    str x28, [x0], #8
    str x29, [x0], #8
    str x30, [x0], #8
    mov x2, sp
    str x2, [x0]

    ldr x19, [x1], #8
    ldr x20, [x1], #8
    ldr x21, [x1], #8
    ldr x22, [x1], #8
    ldr x23, [x1], #8
    ldr x24, [x1], #8
    ldr x25, [x1], #8
    ldr x26, [x1], #8
    ldr x27, [x1], #8
    ldr x28, [x1], #8
    ldr x29, [x1], #8
    ldr x30, [x1], #8
    ldr x1, [x1]
    mov sp, x1
    ret
