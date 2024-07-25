use core::isize;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::VecDeque;
use alloc::{sync::Arc, vec::Vec};
use scheduler::BaseScheduler;

// 静态优先级调度算法，相同优先级使用FIFO。
// 调度算法不会自动调整任务的优先级，但可以手动调整。
pub(super) struct StatPrioTask<T, const PRIO_LEVEL_NUM: usize> {
    inner: T,
    priority: AtomicUsize // 在该struct内保证priority合法
}

impl<T, const N: usize> StatPrioTask<T, N> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            priority: AtomicUsize::new(1),
        }
    }

    pub const fn inner(&self) -> &T {
        &self.inner
    }

    fn set_priority(&self, prio: isize) -> bool {
        if prio >= 0 && prio < N as isize {
            self.priority.store(prio as usize, Ordering::Release);
            true
        }
        else {
            false
        }
    }

    fn get_priority(&self) -> usize {
        self.priority.load(Ordering::Acquire)
    }
}

pub(super) struct StatPrioScheduler<T, const PRIO_LEVEL_NUM: usize> {
    ready_queues: Vec<VecDeque<Arc<StatPrioTask<T, PRIO_LEVEL_NUM>>>>,
}

impl<T, const N: usize> StatPrioScheduler<T, N> {
    /// Creates a new empty [`StatPrioScheduler`].
    pub const fn new() -> Self {
        Self {
            ready_queues: Vec::new()
        }
    }
    /// get the name of scheduler
    pub fn scheduler_name() -> &'static str {
        "Static Priority"
    }
}

impl<T, const N: usize> StatPrioScheduler<T, N> {
    fn highest_priority(&self) -> usize {
        for priority in 0 .. N {
            if !self.ready_queues[priority].is_empty() {
                return priority;
            }
        }
        N
    }
}

impl<T, const N: usize> BaseScheduler for StatPrioScheduler<T, N> {
    type SchedItem = Arc<StatPrioTask<T, N>>;

    fn init(&mut self) {
        assert!(N > 0);
        assert!(N <= (isize::MAX as usize) + 1);
        for _ in 0 .. N {
            self.ready_queues.push(VecDeque::new());
        }
    }

    fn add_task(&mut self, task: Self::SchedItem) {
        self.put_prev_task(task, false)
    }

    // 需要保证：每个任务只在调度器中存储了一个实例。即，调度器中不会有多个Arc指向同一任务。
    fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        for priority in 0 .. N {
            for index in 0 .. self.ready_queues[priority].len() {
                if Arc::ptr_eq(&self.ready_queues[priority][index], task) {
                    return self.ready_queues[priority].remove(index);
                }
            }
        }
        None
    }

    fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
        let mut return_task: Option<Self::SchedItem> = None;
        for priority in 0 .. N {
            return_task = self.ready_queues[priority].pop_front();
            if return_task.is_some() {
                break;
            }
        }
        return_task
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, preempt: bool) {
        let priority: usize = prev.get_priority();
        if preempt {
            self.ready_queues[priority].push_front(prev);
        }
        else {
            self.ready_queues[priority].push_back(prev);
        }
    }

    fn task_tick(&mut self, current: &Self::SchedItem) -> bool {
        let current_prio = current.get_priority();
        let self_prio = self.highest_priority();
        self_prio > current_prio
    }

    fn set_priority(&mut self, task: &Self::SchedItem, prio: isize) -> bool {
        task.set_priority(prio)
    }
}