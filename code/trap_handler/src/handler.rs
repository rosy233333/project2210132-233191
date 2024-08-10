use core::{cell::UnsafeCell, mem::ManuallyDrop};

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use lazy_init::LazyInit;
use riscv::register::{scause::{self, Exception, Interrupt, Trap}, stval};
use spinlock::SpinNoIrq;
use task_management::TaskContext;

#[cfg(feature = "log")]
use axlog::debug;

pub(crate) struct HandlerMap<T: Sync> {
    map: UnsafeCell<BTreeMap<usize, T>>,
    default: T,
    write_lock: SpinNoIrq<usize>,
}

impl<T: Sync> HandlerMap<T> {
    pub(crate) fn new(default: T) -> Self {
        Self {
            map: UnsafeCell::new(BTreeMap::new()),
            default,
            write_lock: SpinNoIrq::new(0),
        }
    }

    pub(crate) fn insert(&self, key: usize, value: T) {
        let lock = self.write_lock.lock();
        unsafe { (&mut *self.map.get()).insert(key, value); }
        drop(lock);
    }

    pub(crate) fn remove(&self, key: usize) {
        let lock = self.write_lock.lock();
        unsafe { (&mut *self.map.get()).remove(&key); }
        drop(lock);
    }

    pub(crate) fn get_ref(&self, key: usize) -> &T {
        unsafe { (&*self.map.get()).get(&key).or(Some(&self.default)).unwrap() }
    }
}

unsafe impl<T: Sync> Sync for HandlerMap<T> { }

pub(crate) static INTERRUPT_HANDLER: LazyInit<HandlerMap<Box<dyn Fn(usize, &mut TaskContext) + Send + Sync>>> = LazyInit::new();
pub(crate) static EXCEPTION_HANDLER: LazyInit<HandlerMap<Box<dyn Fn(usize, &mut TaskContext) + Send + Sync>>> = LazyInit::new();
pub(crate) static EXTINTR_HANDLER: LazyInit<HandlerMap<Box<dyn Fn() + Send + Sync>>> = LazyInit::new();
pub(crate) static SYSCALL_HANDLER: LazyInit<HandlerMap<Box<dyn (Fn([usize; 6]) -> usize) + Send + Sync>>> = LazyInit::new();

pub(crate) fn init_handler() {
    INTERRUPT_HANDLER.init_by(HandlerMap::new(Box::new(|_stval: usize, _context|{
        panic!("Unhandled interrupt!");
    })));
    EXCEPTION_HANDLER.init_by(HandlerMap::new(Box::new(|_stval: usize, _context|{
        panic!("Unhandled exception!");
    })));
    EXTINTR_HANDLER.init_by(HandlerMap::new(Box::new(| |{
        panic!("Unhandled external interrupt!");
    })));
    SYSCALL_HANDLER.init_by(HandlerMap::new(Box::new(|_args|{
        panic!("Unhandled system call!");
    })));

    INTERRUPT_HANDLER.insert(Interrupt::SupervisorExternal.try_into().unwrap(), Box::new(|_stval, _context| {
        let irq_num: usize = unimplemented!(); // 从plic处获取中断号

        #[cfg(feature = "log")]
        debug!("New external interrupt, irq_num: {}", irq_num);

        EXTINTR_HANDLER.get_ref(irq_num)();
    }));

    EXCEPTION_HANDLER.insert(Exception::UserEnvCall.try_into().unwrap(), Box::new(|_stval, context| {
        context.step_sepc();
        let syscall_num = context.get_syscall_num();
        let syscall_args = context.get_syscall_args();

        #[cfg(feature = "log")]
        debug!("New syscall, syscall_num: {}, syscall_args: {:?}", syscall_num, syscall_args);

        let ret_value = SYSCALL_HANDLER.get_ref(syscall_num)(syscall_args);
        context.set_ret_code(ret_value);
    }));
}

#[no_mangle]
pub(crate) fn trap_handler(trap_context: &mut TaskContext) {
    let scause = scause::read();
    let stval = stval::read();

    #[cfg(feature = "log")]
    debug!("New trap, scause: {:#016x}, stval: {:#016x}, sepc: {:#016x}", scause.bits(), stval, trap_context.sepc);

    match scause.cause() {
        Trap::Interrupt(interrupt) => {
            let cause: usize = interrupt.try_into().unwrap();
            INTERRUPT_HANDLER.get_ref(cause)(stval, trap_context);
        },
        Trap::Exception(exception) => {
            let cause: usize = exception.try_into().unwrap();
            EXCEPTION_HANDLER.get_ref(cause)(stval, trap_context);
        }
    }

    #[cfg(feature = "preempt")]
    unimplemented!()
}