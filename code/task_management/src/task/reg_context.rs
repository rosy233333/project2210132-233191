use core::arch::asm;

use riscv::register::sstatus::{self, Sstatus};

/// General registers of RISC-V.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegisters {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize, // only valid for user traps
    pub tp: usize, // only valid for user traps
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

/// Saved registers when a trap (interrupt or exception) occurs.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskContext {
    /// All general registers.
    pub regs: GeneralRegisters,
    /// Supervisor Exception Program Counter.
    pub sepc: usize,
    /// Supervisor Status Register.
    pub sstatus: usize,
    /// 浮点数寄存器
    pub fs: [usize; 2],
}

impl TaskContext {
    // pub fn set_user_sp(&mut self, user_sp: usize) {
    //     self.regs.sp = user_sp;
    // }

    // /// 用于第一次进入应用程序时的初始化
    // pub fn app_init_context(app_entry: usize, user_sp: usize) -> Self {
    //     let sstatus = sstatus::read();
    //     // 当前版本的riscv不支持使用set_spp函数，需要手动修改
    //     // 修改当前的sstatus为User，即是第8位置0
    //     let mut trap_frame = TaskContext::default();
    //     trap_frame.set_user_sp(user_sp);
    //     trap_frame.sepc = app_entry;
    //     trap_frame.sstatus =
    //         unsafe { (*(&sstatus as *const Sstatus as *const usize) & !(1 << 8)) & !(1 << 1) };
    //     unsafe {
    //         // a0为参数个数
    //         // a1存储的是用户栈底，即argv
    //         trap_frame.regs.a0 = *(user_sp as *const usize);
    //         trap_frame.regs.a1 = *(user_sp as *const usize).add(1);
    //     }
    //     trap_frame
    // }

    /// 设置返回值
    pub fn set_ret_code(&mut self, ret_value: usize) {
        self.regs.a0 = ret_value;
    }

    /// 设置TLS
    pub fn set_tls(&mut self, tls_value: usize) {
        self.regs.tp = tls_value;
    }

    /// 获取 sp
    pub fn get_sp(&self) -> usize {
        self.regs.sp
    }

    /// 设置 pc
    pub fn set_pc(&mut self, pc: usize) {
        self.sepc = pc;
    }

    /// pc 倒退到 syscall 指令的长度
    pub fn rewind_pc(&mut self) {
        self.sepc -= 4;
    }

    /// 设置 arg0
    pub fn set_arg0(&mut self, arg: usize) {
        self.regs.a0 = arg;
    }

    /// 设置 arg1
    pub fn set_arg1(&mut self, arg: usize) {
        self.regs.a1 = arg;
    }

    /// 设置 arg2
    pub fn set_arg2(&mut self, arg: usize) {
        self.regs.a2 = arg;
    }

    /// 获取 pc
    pub fn get_pc(&self) -> usize {
        self.sepc
    }

    /// 获取 ret
    pub fn get_ret_code(&self) -> usize {
        self.regs.a0
    }

    /// 设置返回地址
    pub fn set_ra(&mut self, ra: usize) {
        self.regs.ra = ra;
    }

    /// 获取所有 syscall 参数
    pub fn get_syscall_args(&self) -> [usize; 6] {
        [
            self.regs.a0,
            self.regs.a1,
            self.regs.a2,
            self.regs.a3,
            self.regs.a4,
            self.regs.a5,
        ]
    }

    /// 获取 syscall id
    pub fn get_syscall_num(&self) -> usize {
        self.regs.a7 as _
    }
}

impl TaskContext {
    /// Creates a new default context for a new task.
    pub const fn new() -> Self {
        unsafe { core::mem::MaybeUninit::zeroed().assume_init() }
    }

    // /// Initializes the context for a new task, with the given entry point and
    // /// kernel stack.
    // pub fn init(&mut self, entry: usize, kstack_top: usize, tls_area: usize) {
    //     self.regs.sp = kstack_top;
    //     self.regs.ra = entry;
    //     self.regs.tp = tls_area;
    //     // #[cfg(not(feature = "async"))] {
    //     //     self.sp = kstack_top.as_usize();
    //     //     self.ra = entry;
    //     //     self.tp = tls_area.as_usize();
    //     // }
    // }
}

#[cfg(target_arch = "riscv32")]
core::arch::global_asm!(
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
core::arch::global_asm!(
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

use core::ptr::NonNull;
use super::switch::schedule_with_sp_change;

const TASKCONTEXT_SIZE: usize = core::mem::size_of::<TaskContext>();

#[naked]
// Save the previous context to the stack, and call schedule_with_sp_change().
pub(crate) unsafe extern "C" fn save_prev_ctx(prev_ctx_ref: &mut NonNull<TaskContext>, s_irq_flag: usize) {
    core::arch::asm!(
        // 参考AsyncStarry的crates/axtrap/src/arch/riscv/trap.S
        // 在栈上申请空间并移动sp（sp的原值借助sscratch间接存储在TaskContext中）
        "
        csrrw   x0, sscratch, sp
        addi    sp, sp, -{taskctx_size}
        ",
        // 存储通用寄存器
        "
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
        STR     sp, sp, 1
        ",
        // 存储特殊寄存器
        // or      t1, t1, a1 是为了恢复原本的中断使能位
        "
        csrr    t0, sepc
        csrr    t1, sstatus
        or      t1, t1, a1
        csrrw   t2, sscratch, zero
        STR     t0, sp, 31
        STR     t1, sp, 32
        STR     t2, sp, 1
        .short  0xa622
        .short  0xaa26
        ",
        // a0 -> ctx_ref
        // sp -> *mut TaskContext
        "STR     sp, a0, 0",
        "call {schedule_with_sp_change}",
        // // The stack has changed, if the next task is a coroutine, the execution will return to here.
        // // But the ra is not correct.
        // "ret",
        taskctx_size = const TASKCONTEXT_SIZE,
        schedule_with_sp_change = sym schedule_with_sp_change,
        options(noreturn),
    )
}

#[naked]
/// Load the next context from the stack.
pub(crate) unsafe extern "C" fn load_next_ctx(next_ctx_ref: &mut NonNull<TaskContext>) {
    core::arch::asm!(
        "LDR     sp, a0, 0",
        "li      a1, 8",
        "STR     a1, a0, 0",
        "
        LDR     t0, sp, 31
        LDR     t1, sp, 32
        csrw    sepc, t0
        csrw    sstatus, t1
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
        ",
        // TODO: 这句原本是sret，用于中断返回。之后添加中断支持后，将线程yield与中断打断结合，再改回sret
        "
        ret
        ",
        options(noreturn),
    )
}