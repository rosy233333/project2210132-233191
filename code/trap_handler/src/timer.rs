use core::borrow::Borrow;

use axlog::debug;
use lazy_init::LazyInit;
use riscv::register::{scause::{Interrupt, Trap}, time};

use crate::{handler::INTERRUPT_HANDLER, register_trap_handler};

#[crate_interface::def_interface]
pub trait CurrentTimebaseFrequency {
    /// 获取当前CPU的时基频率（timebase frequency，即time寄存器递增的频率，单位Hz）
    /// 该频率会在CPU的运行过程中保持不变，因此该crate会在每个CPU上调用一次该函数并存储结果，之后就使用存储的值。
    fn current_timebase_frequency() -> usize;
}

#[cfg(feature = "smp")]
#[percpu::def_percpu]
static TIMEBASE_FREQUENCY: LazyInit<usize> = LazyInit::new();

#[cfg(not(feature = "smp"))]
static TIMEBASE_FREQUENCY: LazyInit<usize> = LazyInit::new();

/// 时钟中断触发的频率（Hz）
static TIMER_FREQUENCY: usize = 1000_000;

// 在init_handler()之后，enable_irqs()之前调用
pub(crate) fn init_timer_on_main_processor() {
    // 因为引用的task_management模块里会进行percpu的初始化，因此该模块不需要初始化percpu。
    #[cfg(feature = "smp")]
    TIMEBASE_FREQUENCY.with_current(|tf| tf.init_by(crate_interface::call_interface!(CurrentTimebaseFrequency::current_timebase_frequency())));

    #[cfg(not(feature = "smp"))]
    TIMEBASE_FREQUENCY.init_by(crate_interface::call_interface!(current_timebase_frequency()));

    register_trap_handler(Trap::Interrupt(Interrupt::SupervisorTimer), timer_interrupt_handler);
    sbi_rt::set_timer(0);
}

#[cfg(feature = "smp")]
pub(crate) fn init_timer_on_secondary_processor() {
    TIMEBASE_FREQUENCY.with_current(|tf| tf.init_by(crate_interface::call_interface!(CurrentTimebaseFrequency::current_timebase_frequency())));
    sbi_rt::set_timer(0);
}

fn timer_interrupt_handler(_stval: usize) {
    #[cfg(feature = "smp")]
    let timebase_frequency: usize = TIMEBASE_FREQUENCY.with_current(|tf| **tf);
    #[cfg(not(feature = "smp"))]
    let timebase_frequency: usize = *TIMEBASE_FREQUENCY;

    let now = time::read();
    let next_deadline = now + timebase_frequency / TIMER_FREQUENCY;
    sbi_rt::set_timer(next_deadline as u64);

    // 时钟中断处理函数的实际功能
    // #[cfg(feature = "log")]
    // debug!("Receive timer interrupt!");
}