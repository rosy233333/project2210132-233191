// -----初始化-----

use alloc::boxed::Box;
use riscv::register::{scause::Trap, sie, sstatus};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::{entry::set_stvec, handler::{init_handler, EXCEPTION_HANDLER, EXTINTR_HANDLER, INTERRUPT_HANDLER, SYSCALL_HANDLER}};

pub use crate::entry::TaskContext;

#[cfg(feature = "timer")]
use crate::timer::init_timer_on_main_processor;
#[cfg(all(feature = "timer", feature = "smp"))]
use crate::timer::init_timer_on_secondary_processor;
#[cfg(feature = "timer")]
pub use crate::timer::CurrentTimebaseFrequency;

#[cfg(feature = "smp")]
static MAIN_PROCESSOR_INIT_FINISHED: AtomicBool = AtomicBool::new(false);

/// 主处理器的初始化，需要先使用这里的接口注册几个中断的处理函数，再设置stvec寄存器，最后打开中断。
pub fn init_main_processor() {
    init_handler();
    set_stvec();
    unsafe {
        sie::set_sext();
        sie::set_ssoft();
        sie::set_stimer();
    }
    #[cfg(feature = "timer")]
    init_timer_on_main_processor();
    // enable_irqs();

    #[cfg(feature = "smp")]
    MAIN_PROCESSOR_INIT_FINISHED.store(true, Ordering::Release);
}

/// 副处理器的初始化，只需设置stvec寄存器和打开中断。
/// 需要在主处理器初始化完成后调用。
#[cfg(feature = "smp")]
pub fn init_secondary_processor() {

    while !MAIN_PROCESSOR_INIT_FINISHED.load(Ordering::Acquire) { } //等待主CPU初始化完成

    set_stvec();
    unsafe {
        sie::set_sext();
        sie::set_ssoft();
        sie::set_stimer();
    }
    #[cfg(feature = "timer")]
    init_timer_on_secondary_processor();
    // enable_irqs();
}

#[no_mangle]
pub fn enable_irqs() {
    unsafe {
        sstatus::set_sie();
    }
}

pub fn disable_irqs() {
    unsafe {
        sstatus::clear_sie();
    }
}

// -----注册处理程序-----

/// 根据trap原因注册trap处理程序。
/// 其中外部中断和系统调用的处理已经通过该函数注册，用户需要注册的是具体的中断号/系统调用号的处理。
/// trap --> 根据scause判断
///     Interrupt(SupervisorExternal) --> 获取irq_num，进入对应的interrupt_handler
///     Exception(UserEnvCall) --> 获取系统调用号和参数，进入对应的syscall_handler
///     其它 --> 进入对应的trap_handler
pub fn register_trap_handler<F>(scause: Trap, handler: F)
where F: Fn(usize, &mut TaskContext) + Send + Sync + 'static {
    match scause {
        Trap::Interrupt(interrupt) => INTERRUPT_HANDLER.insert(interrupt.try_into().unwrap(), Box::new(handler)),
        Trap::Exception(exception) => EXCEPTION_HANDLER.insert(exception.try_into().unwrap(), Box::new(handler)),
    }
}

/// 根据中断号，注册外部中断处理程序
/// 注意：使用register_trap_handler函数注册Interrupt(SupervisorExternal)的trap_handler会覆盖使用该函数注册的处理程序。
pub fn register_extintr_handler<F>(irq_num: usize, handler: F)
where F: Fn() + Send + Sync + 'static {
    EXTINTR_HANDLER.insert(irq_num, Box::new(handler));
}

/// 根据系统调用号，注册系统调用处理程序
/// 注意：使用register_trap_handler函数注册Exception(UserEnvCall)的trap_handler会覆盖使用该函数注册的处理程序。
pub fn register_syscall_handler<F>(sc_num: usize, handler: F)
where F: (Fn([usize; 6]) -> usize) + Send + Sync + 'static {
    SYSCALL_HANDLER.insert(sc_num, Box::new(handler));
}