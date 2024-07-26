use core::{future::Future, mem::ManuallyDrop, pin::Pin, ptr::NonNull, sync::atomic::{AtomicI32, AtomicU64, Ordering}, task::Poll};
use alloc::{boxed::Box, sync::Arc};
use spinlock::{SpinNoIrq, SpinNoIrqGuard};
use crossbeam::atomic::AtomicCell;
use task_queues::scheduler::AxTask;

mod reg_context;
mod switch;
mod waker;

pub(crate) use reg_context::TaskContext;
pub(crate) use switch::{preempt_switch_entry, switch_entry};

use crate::{exit_current, exit_current_async, processor::Processor};

pub type Task = AxTask<TaskInner>;

pub struct TaskInner {
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

    // 目前不考虑
    // /// CPU亲和性
    // /// 用位图存储
    // cpu_set: AtomicU64,

    /// ---上下文相关---
    /// The future of the async task.
    future: Option<AtomicCell<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>>, // 取值为Some还是None是在创建时确定的，因此Option在AtomicCell外部

    /// When the async task is breaked by the interrupt,
    /// this field will be valid. Otherwise, it is dangerous to access this field.
    /// 我打算在我的模块中同时支持线程和协程。线程总是使用ctx_ref，而不使用future。
    ctx_ref: AtomicCell<NonNull<TaskContext>>,
}

/// The possible states of a task.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs)]
pub(crate) enum TaskState {
    /// 该状态可能正在执行，也可能就绪
    Runable = 1,
    /// 设置Blocking状态 -> 加入阻塞队列 -> 保存上下文 -> 设置Blocked状态
    Blocking = 2,
    /// 设置Blocking状态 -> 加入阻塞队列 -> 保存上下文 -> 设置Blocked状态
    Blocked = 3,
    /// 保存返回值 -> 设置Exited状态 -> 停止执行
    Exited = 4,
}

/// A unique identifier for a thread.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TaskId(u64);

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TaskId {
    /// Create a new task ID.
    pub(crate) fn new() -> Self {
        Self(ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Convert the task ID to a `u64`.
    pub(crate) const fn as_u64(&self) -> u64 {
        self.0
    }

    // #[cfg(feature = "monolithic")]
    // /// 清空计数器，为了给单元测试使用
    // /// 保留了gc, 主调度，内核进程
    // pub fn clear() {
    //     ID_COUNTER.store(5, Ordering::Relaxed);
    // }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

// 分别代表用于创建线程和协程的同步和异步函数
// pub trait TaskFunc: (FnOnce() -> i32) + Send + 'static { }
// pub trait AsyncTaskFunc: Future<Output = i32> + Send + 'static { }

unsafe impl Send for TaskInner {}
unsafe impl Sync for TaskInner {}

/// 访问各个成员的方法
impl TaskInner {
    #[inline]
    pub(crate) fn id(&self) -> u64 {
        self.id.as_u64()
    }

    #[inline]
    /// lock the task state and ctx_ptr access
    pub(crate) fn state_lock_manual(&self) -> ManuallyDrop<SpinNoIrqGuard<TaskState>> {
        ManuallyDrop::new(self.state.lock())
    }

    #[inline]
    /// lock the task state and ctx_ptr access
    pub(crate) fn state_lock(&self) -> SpinNoIrqGuard<TaskState> {
        self.state.lock()
    }

    #[inline]
    /// get the state of the task
    pub(crate) fn state(&self) -> TaskState {
        *self.state.lock()
    }

    #[inline]
    /// set the state of the task
    pub(crate) fn set_state(&self, state: TaskState) {
        *self.state.lock() = state
    }

    /// Whether the task is Exited
    #[inline]
    pub(crate) fn is_exited(&self) -> bool {
        matches!(self.state(), TaskState::Exited)
    }

    /// Whether the task is runnalbe
    #[inline]
    pub(crate) fn is_runable(&self) -> bool {
        matches!(self.state(), TaskState::Runable)
    }

    /// Whether the task is blocking
    #[inline]
    pub(crate) fn is_blocking(&self) -> bool {
        matches!(self.state(), TaskState::Blocking)
    }

    /// Whether the task is blocked
    #[inline]
    pub(crate) fn is_blocked(&self) -> bool {
        matches!(self.state(), TaskState::Blocked)
    }

    #[inline]
    pub(crate) fn set_exit_code(&self, exit_code: i32) {
        self.exit_code.store(exit_code, Ordering::Release)
    }
}

/// pub(crate)方法
impl TaskInner {
    pub(crate) fn new<F>(func: F) -> Arc<Task>
    where F: (FnOnce() -> i32) + Send + 'static {
        Self::new_raw(func, false, false)
    }

    pub(crate) fn new_async<F>(func: F) -> Arc<Task>
    where F: Future<Output = i32> + Send + 'static {
        Self::new_async_raw(func, false, false)
    }

    pub(crate) fn new_idle() -> Arc<Task> {
        struct IdleTaskFuture();
        impl Future for IdleTaskFuture {
            type Output = i32;
        
            fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<i32> {
                Poll::Pending
            }
        }
        Self::new_async_raw(IdleTaskFuture { }, true, false)
    }

    pub(crate) fn new_init<F>(func: F) -> Arc<Task>
    where F: (FnOnce() -> i32) + Send + 'static {
        Self::new_raw(func, false, true)
    }

    pub(crate) fn new_async_init<F>(func: F) -> Arc<Task>
    where F: Future<Output = i32> + Send + 'static {
        Self::new_async_raw(func, false, true)
    }

    pub(crate) fn wakeup(self: Arc<AxTask<Self>>) {
        let mut state = self.state_lock_manual();
        match **state {
            TaskState::Blocking => **state = TaskState::Runable,
            TaskState::Runable => (),
            TaskState::Blocked => {
                // debug!("task unblock: {}", self.id());
                **state = TaskState::Runable;
                ManuallyDrop::into_inner(state);
                // may be other processor wake up
                Processor::with_current(|processor| {
                    processor.add_task_to_local(self);
                });
                return;
            }
            _ => panic!("unexpect state when wakeup_task"),
        }
        ManuallyDrop::into_inner(state);
    }
}

/// private方法
impl TaskInner {
    fn new_raw<F>(func: F, is_idle: bool, is_init: bool) -> Arc<Task>
    where F: (FnOnce() -> i32) + Send + 'static {
        Self::new_async_raw_with_wrapped_func(async { // 将线程转化为协程，从而规避线程与协程的启动方式不同的问题 
            let exit_code = func();
            exit_current(exit_code); // 将任务的自然退出方式也统一为使用exit系列函数
        }, is_idle, is_init)
    }

    fn new_async_raw<F>(func: F, is_idle: bool, is_init: bool) -> Arc<Task>
    where F: Future<Output = i32> + Send + 'static {
        Self::new_async_raw_with_wrapped_func(async { 
            let exit_code = func.await;
            exit_current_async(exit_code).await; // 将任务的自然退出方式也统一为使用exit系列函数。结果：直属于TaskInner的Future不会返回Ready，只会返回Pending。
        }, is_idle, is_init)
    }

    fn new_async_raw_with_wrapped_func<F>(func: F, is_idle: bool, is_init: bool) -> Arc<Task>
    where F: Future<Output = ()> + Send + 'static {
        Arc::new(Task::new(TaskInner {
            id: TaskId::new(),
            is_idle,
            is_init,
            state: SpinNoIrq::new(TaskState::Runable),
            exit_code: AtomicI32::new(0),
            future: Some(AtomicCell::new(Box::pin(func))),
            ctx_ref: AtomicCell::new(NonNull::dangling()),
        }))
    }
}