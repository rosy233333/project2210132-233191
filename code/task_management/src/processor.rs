use core::cell::UnsafeCell;

/// 我的CPU核心设计，应该不需要exit_task队列和gc任务？
/// 因为任务退出后不能立即释放的情况，应该就是其它任务join了它的情况。
/// 但在我的设计中，join的任务会拥有被join的任务的Arc，因此被join的任务不会提前释放？只有等到所有join的任务都获得了返回值，释放了持有的Arc后，被join的任务才会被释放？
/// 访问Processor相关内容时，必须关中断，否则任务在访问当前处理器时被抢占，之后可能会调度到其它处理器上继续运行，从而出现错误
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
    idle_task: Arc<Task>
}