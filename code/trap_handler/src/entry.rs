use core::{arch::global_asm, mem::size_of};

use riscv::register::{sie, sstatus, stvec};
use task_management::TaskContext;
use crate::handler::trap_handler;

#[cfg(target_arch = "riscv32")]
global_asm!(
    r"
.ifndef XLENB
.equ XLENB, 4

.macro LDR rd, rs, off
    lw \rd, \off*XLENB(\rs)
.endm
.macro STR rs2, rs1, off
    sw \rs2, \off*XLENB(\rs1)
.endm

.endif"
);

#[cfg(target_arch = "riscv64")]
global_asm!(
    r"
.ifndef XLENB
.equ XLENB, 8

.macro LDR rd, rs, off
    ld \rd, \off*XLENB(\rs)
.endm
.macro STR rs2, rs1, off
    sd \rs2, \off*XLENB(\rs1)
.endm

.endif",
);

#[cfg(feature = "fp_context")]
global_asm!(
// 将当前的CPU上下文以trap上下文形式存储于sp指向的TaskContext结构中
// TaskContext结构的sp字段将保存sscratch寄存器的值
"
.macro SAVE_TRAP_CTX_
    STR     ra, sp, 0
    STR     t0, sp, 4
    STR     t1, sp, 5
    STR     t2, sp, 6
    STR     s0, sp, 7
    STR     s1, sp, 8
    STR     a0, sp, 9
    STR     a1, sp, 10
    STR     a2, sp, 11
    STR     a3, sp, 12
    STR     a4, sp, 13
    STR     a5, sp, 14
    STR     a6, sp, 15
    STR     a7, sp, 16
    STR     s2, sp, 17
    STR     s3, sp, 18
    STR     s4, sp, 19
    STR     s5, sp, 20
    STR     s6, sp, 21
    STR     s7, sp, 22
    STR     s8, sp, 23
    STR     s9, sp, 24
    STR     s10, sp, 25
    STR     s11, sp, 26
    STR     t3, sp, 27
    STR     t4, sp, 28
    STR     t5, sp, 29
    STR     t6, sp, 30
    csrrw   t2, sscratch, zero
    STR     t2, sp, 1

    csrr    t0, sepc
    csrr    t1, sstatus
    STR     t0, sp, 31
    STR     t1, sp, 32
",
"
    .short  0xa622
    .short  0xaa26
",
"
.endm
",
// 从sp指向的TaskContext结构中恢复上下文，无论其为线程上下文还是trap上下文
// 该宏的执行会改变sp的值
"
.macro LOAD_CTX_AND_RETURN_
    LDR     t0, sp, 31
    LDR     t1, sp, 32
    beqz    t0, 2f

    csrw    sepc, t0
    csrw    sstatus, t1
",
"
    .short  0x2432
    .short  0x24d2
",
"
    LDR     ra, sp, 0
    LDR     t0, sp, 4
    LDR     t1, sp, 5
    LDR     t2, sp, 6
    LDR     s0, sp, 7
    LDR     s1, sp, 8
    LDR     a0, sp, 9
    LDR     a1, sp, 10
    LDR     a2, sp, 11
    LDR     a3, sp, 12
    LDR     a4, sp, 13
    LDR     a5, sp, 14
    LDR     a6, sp, 15
    LDR     a7, sp, 16
    LDR     s2, sp, 17
    LDR     s3, sp, 18
    LDR     s4, sp, 19
    LDR     s5, sp, 20
    LDR     s6, sp, 21
    LDR     s7, sp, 22
    LDR     s8, sp, 23
    LDR     s9, sp, 24
    LDR     s10, sp, 25
    LDR     s11, sp, 26
    LDR     t3, sp, 27
    LDR     t4, sp, 28
    LDR     t5, sp, 29
    LDR     t6, sp, 30
    LDR     sp, sp, 1

    sret

2:
",
"
    .short  0x2432
    .short  0x24d2
",
"
    LDR     ra, sp, 0
    LDR     s0, sp, 7
    LDR     s1, sp, 8
    LDR     s2, sp, 17
    LDR     s3, sp, 18
    LDR     s4, sp, 19
    LDR     s5, sp, 20
    LDR     s6, sp, 21
    LDR     s7, sp, 22
    LDR     s8, sp, 23
    LDR     s9, sp, 24
    LDR     s10, sp, 25
    LDR     s11, sp, 26
    LDR     sp, sp, 1

    ret

.endm
",
// 中断处理入口
"
.section .text
.balign 4
.global trap_entry
trap_entry:
    csrw    sscratch, sp
    addi    sp, sp, -{taskctx_size}
    SAVE_TRAP_CTX_
    mv      a0, sp
    call    {trap_handler}
    LOAD_CTX_AND_RETURN_
",
taskctx_size = const TASKCONTEXT_SIZE,
trap_handler = sym trap_handler,

);

#[cfg(not(feature = "fp_context"))]
global_asm!(
// 将当前的CPU上下文以trap上下文形式存储于sp指向的TaskContext结构中
// TaskContext结构的sp字段将保存sscratch寄存器的值
"
.macro SAVE_TRAP_CTX_
    STR     ra, sp, 0
    STR     t0, sp, 4
    STR     t1, sp, 5
    STR     t2, sp, 6
    STR     s0, sp, 7
    STR     s1, sp, 8
    STR     a0, sp, 9
    STR     a1, sp, 10
    STR     a2, sp, 11
    STR     a3, sp, 12
    STR     a4, sp, 13
    STR     a5, sp, 14
    STR     a6, sp, 15
    STR     a7, sp, 16
    STR     s2, sp, 17
    STR     s3, sp, 18
    STR     s4, sp, 19
    STR     s5, sp, 20
    STR     s6, sp, 21
    STR     s7, sp, 22
    STR     s8, sp, 23
    STR     s9, sp, 24
    STR     s10, sp, 25
    STR     s11, sp, 26
    STR     t3, sp, 27
    STR     t4, sp, 28
    STR     t5, sp, 29
    STR     t6, sp, 30
    csrrw   t2, sscratch, zero
    STR     t2, sp, 1

    csrr    t0, sepc
    csrr    t1, sstatus
    STR     t0, sp, 31
    STR     t1, sp, 32
",
// "
//     .short  0xa622
//     .short  0xaa26
// ",
"
.endm
",
// 从sp指向的TaskContext结构中恢复上下文，无论其为线程上下文还是trap上下文
// 该宏的执行会改变sp的值
"
.macro LOAD_CTX_AND_RETURN_
    LDR     t0, sp, 31
    LDR     t1, sp, 32
    beqz    t0, 2f

    csrw    sepc, t0
    csrw    sstatus, t1
",
// "
//     .short  0x2432
//     .short  0x24d2
// ",
"
    LDR     ra, sp, 0
    LDR     t0, sp, 4
    LDR     t1, sp, 5
    LDR     t2, sp, 6
    LDR     s0, sp, 7
    LDR     s1, sp, 8
    LDR     a0, sp, 9
    LDR     a1, sp, 10
    LDR     a2, sp, 11
    LDR     a3, sp, 12
    LDR     a4, sp, 13
    LDR     a5, sp, 14
    LDR     a6, sp, 15
    LDR     a7, sp, 16
    LDR     s2, sp, 17
    LDR     s3, sp, 18
    LDR     s4, sp, 19
    LDR     s5, sp, 20
    LDR     s6, sp, 21
    LDR     s7, sp, 22
    LDR     s8, sp, 23
    LDR     s9, sp, 24
    LDR     s10, sp, 25
    LDR     s11, sp, 26
    LDR     t3, sp, 27
    LDR     t4, sp, 28
    LDR     t5, sp, 29
    LDR     t6, sp, 30
    LDR     sp, sp, 1

    sret

2:
",
// "
//     .short  0x2432
//     .short  0x24d2
// ",
"
    LDR     ra, sp, 0
    LDR     s0, sp, 7
    LDR     s1, sp, 8
    LDR     s2, sp, 17
    LDR     s3, sp, 18
    LDR     s4, sp, 19
    LDR     s5, sp, 20
    LDR     s6, sp, 21
    LDR     s7, sp, 22
    LDR     s8, sp, 23
    LDR     s9, sp, 24
    LDR     s10, sp, 25
    LDR     s11, sp, 26
    LDR     sp, sp, 1

    ret

.endm
",
// 中断处理入口
"
.section .text
.balign 4
.global trap_entry
trap_entry:
    csrw    sscratch, sp
    addi    sp, sp, -{taskctx_size}
    SAVE_TRAP_CTX_
    mv      a0, sp
    call    {trap_handler}
    LOAD_CTX_AND_RETURN_
",
taskctx_size = const TASKCONTEXT_SIZE,
trap_handler = sym trap_handler,

);

const TASKCONTEXT_SIZE: usize = size_of::<TaskContext>();

extern "C" {
    fn trap_entry();
}

pub(crate) fn set_stvec() {
    unsafe {
        stvec::write(trap_entry as usize, stvec::TrapMode::Direct);
    }
}