use core::{cell::UnsafeCell, future::Future, ptr::NonNull, sync::atomic::{AtomicI32, AtomicU64}};
use spinlock::SpinNoIrq;
use crossbeam::atomic::AtomicCell;

pub(crate) type Task = scheduler::FifoTask<TaskInner>;

pub(crate) struct TaskInner {
    // -----不可变属性-----
    /// id
    id: TaskId,
    /// Whether the task is the idle task
    is_idle: bool,
    /// Whether the task is the initial task
    ///
    /// If the task is the initial task, the kernel will terminate
    /// when the task exits.
    is_init: bool,

    // -----可变属性-----

    // 目前不考虑
    // ///---抢占相关---
    // #[cfg(feature = "preempt")]
    // /// Whether the task needs to be rescheduled
    // ///
    // /// When the time slice is exhausted, it needs to be rescheduled
    // need_resched: AtomicBool,
    // #[cfg(feature = "preempt")]
    // /// The disable count of preemption
    // ///
    // /// When the task get a lock which need to disable preemption, it
    // /// will increase the count. When the lock is released, it will
    // /// decrease the count.
    // ///
    // /// Only when the count is zero, the task can be preempted.
    // preempt_disable_count: AtomicUsize,

    /// Task state
    state: SpinNoIrq<TaskState>,

    /// 返回值
    exit_code: AtomicI32,

    /// CPU亲和性
    /// 用位图存储
    cpu_set: AtomicU64,

    /// ---上下文相关---
    /// The future of the async task.
    future: Option<AtomicCell<Pin<Box<dyn Future<Output = i32> + 'static + Send>>>>,

    /// When the async task is breaked by the interrupt,
    /// this field will be valid. Otherwise, it is dangerous to access this field.
    /// 我打算在我的模块中同时支持线程和协程。线程总是使用ctx_ref，而不使用future。
    ctx_ref: UnsafeCell<NonNull<TaskContext>>,
}