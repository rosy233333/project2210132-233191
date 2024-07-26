use core::cell::UnsafeCell;

use alloc::sync::Arc;
use lazy_init::LazyInit;
use spinlock::{SpinNoIrq, SpinNoIrqGuard};
use task_queues::scheduler::{self, BaseScheduler};

use crate::{stack::StackPool, task::{TaskContext, TaskInner}, Task};

#[cfg(feature = "smp")]
#[percpu::def_percpu]
static PROCESSOR: LazyInit<SpinNoIrq<Processor>> = LazyInit::new();

#[cfg(not(feature = "smp"))]
static PROCESSOR: LazyInit<SpinNoIrq<Processor>> = LazyInit::new();

static GLOBAL_SCHEDULER: LazyInit<Arc<SpinNoIrq<Scheduler>>> = LazyInit::new();

pub(crate) struct Processor {
    /// 调度器
    /// 分为局部（当前CPU）调度器和全局（当前地址空间）调度器两级。
    /// CPU获取任务时，优先从局部调度器取出任务，若局部调度器没有任务，则从全局调度器取任务。
    /// 当前任务主动让出（yield_current_to_global除外）或被抢占时，加入局部调度器。
    /// `spawn`、`wake`、`yield`系列方法都提供了将任务加入全局调度器的版本。
    /// 在unikernel下，所有CPU使用同一个静态的全局调度器。之后支持宏内核时，可以将全局调度器与hypervisor、os、进程绑定，在切换hypervisor、os、进程时，同时切换CPU的全局调度器。该设计也使得同一时间，不同核心可以使用不同的全局调度器、运行不同的进程。
    local_scheduler: UnsafeCell<Scheduler>,
    global_scheduler: Arc<SpinNoIrq<Scheduler>>,

    /// 当前任务
    current_task: UnsafeCell<CurrentTask>,

    /// 栈池和当前栈
    stack_pool: UnsafeCell<StackPool>,
    // current_stack: CurrentStack, // 这里要仔细读一下任务切换里栈的行为

    /// 空闲时执行的任务
    idle_task: Arc<Task>,
}

pub(crate) type CurrentTask = task_queues::current::CurrentTask<Task>;
pub(crate) type Scheduler = task_queues::scheduler::Scheduler<TaskInner>;

unsafe impl Sync for Processor {}
unsafe impl Send for Processor {}

/// 访问各个成员的方法
impl Processor {
    // 注意：不要同时申请多个mut引用。
    #[inline]
    pub(crate) fn with_local_scheduler<F, T>(&self, f: F) -> T
    where F: FnOnce(&mut Scheduler) -> T {
        unsafe { f(&mut *self.local_scheduler.get()) }
    }

    // 注意：不要同时申请多个mut引用。
    #[inline]
    pub(crate) fn with_global_scheduler<F, T>(&self, f: F) -> T
    where F: FnOnce(&mut Scheduler) -> T {
        f(&mut self.global_scheduler.lock())
    }

    #[inline]
    pub(crate) fn current_task(&self) -> &mut CurrentTask {
        unsafe { &mut *self.current_task.get() }
    }
}

/// pub(crate) 方法
impl Processor {
    /// 使用percpu库初始化静态变量
    /// 只包含了初始化CPU和调度器的过程，不包含运行main任务
    pub(crate) fn init_main_processor(cpu_id: usize, cpu_num: usize) {
        GLOBAL_SCHEDULER.init_by(Arc::new(SpinNoIrq::new(Scheduler::new())));

        #[cfg(feature = "smp")]
        {
            percpu::init(cpu_num);
            percpu::set_local_thread_pointer(cpu_id);
            PROCESSOR.with_current(|processor| {
                processor.init_by(SpinNoIrq::new(Processor::new()));
            });
        }

        #[cfg(not(feature = "smp"))]
        PROCESSOR.init_by(SpinNoIrq::new(Processor::new()));
    }

    #[cfg(feature = "smp")]
    pub(crate) fn init_secondary_processor(cpu_id: usize) {
        percpu::set_local_thread_pointer(cpu_id);
        PROCESSOR.with_current(|processor| {
            processor.init_by(SpinNoIrq::new(Processor::new()));
        });
    }

    /// 获取当前CPU
    /// 需要在当前核心执行了初始化函数之后调用
    pub(crate) fn with_current<F, T>(f: F) -> T
    where F: FnOnce(&Processor) -> T {
        #[cfg(feature = "smp")]
        {
            PROCESSOR.with_current(|processor| {
                f(&processor.lock())
            })
        }

        #[cfg(not(feature = "smp"))]
        {
            f(&PROCESSOR.lock())
        }
    }

    /// 开始运行任务
    pub(crate) fn run_tasks(&self) -> ! {
        unimplemented!();
    }

    // /// 切换任务
    // pub(crate) fn coroutine_switch(&self) {
    //     // 目前不知道切换函数是否需要全程在PROCESSOR的锁下进行。
    //     // 优点是可以保证整个切换过程不被中断和抢占
    //     // 缺点是切换前后位于两个执行流中，锁的获取和释放存在难点
    //     unimplemented!()
    // }

    // pub(crate) fn thread_switch(&self, prev_ctx: &mut TaskContext) {
    //     unimplemented!()
    // }

    // 只负责加入队列，不负责更改任务状态
    // 应在任务状态更改完成后，再调用该函数
    pub(crate) fn add_task_to_local(&self, task: Arc<Task>) {
        self.with_local_scheduler(|scheduler| {
            scheduler.add_task(task);
        })
    }

    // 只负责加入队列，不负责更改任务状态
    // 应在任务状态更改完成后，再调用该函数
    pub(crate) fn add_task_to_global(&self, task: Arc<Task>) {
        self.with_global_scheduler(|scheduler| {
            scheduler.add_task(task);
        })
    }
}

/// private方法
impl Processor {
    // 需要在GLOBAL_SCHEDULER初始化完成后调用
    fn new() -> Self {
        let idle_task = TaskInner::new_idle();
        Self {
            local_scheduler: UnsafeCell::new(Scheduler::new()),
            global_scheduler: GLOBAL_SCHEDULER.try_get().unwrap().clone(),
            current_task: UnsafeCell::new(CurrentTask::new(idle_task.clone())),
            stack_pool: UnsafeCell::new(StackPool::new()),
            idle_task,
        }
    }
}