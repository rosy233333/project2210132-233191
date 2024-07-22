#[cfg(not(feature = "moic"))]
use alloc::collections::VecDeque;
use alloc::{sync::Arc, vec::Vec};
#[cfg(not(feature = "moic"))]
use spinlock::SpinNoIrq;

pub struct Scheduler<T> {
    #[cfg(not(feature = "moic"))]
    prio_level_num: usize,
    #[cfg(not(feature = "moic"))]
    ready_queues: Vec<SpinNoIrq<VecDeque<Arc<T>>>>,
}

impl<T> Scheduler<T> {
    pub fn new(prio_level_num: usize) -> Scheduler<T> {
        #[cfg(not(feature = "moic"))]
        {
            let mut scheduler = Self {
                #[cfg(not(feature = "moic"))]
                prio_level_num,
                #[cfg(not(feature = "moic"))]
                ready_queues: Vec::new(),
            };
            for _ in 0 .. prio_level_num {
                scheduler.ready_queues.push(SpinNoIrq::new(VecDeque::new()));
            }
            scheduler
        }

        #[cfg(feature = "moic")]
        {

        }
    }

    pub fn add(&mut self, task: Arc<T>, priority: usize) {
        #[cfg(not(feature = "moic"))]
        {
            assert!(priority < self.prio_level_num);
            self.ready_queues[priority].lock().push_back(task);
        }

        #[cfg(feature = "moic")]
        {

        }
    }

    pub fn fetch(&mut self) -> Option<Arc<T>> {
        #[cfg(not(feature = "moic"))]
        {
            let mut return_task: Option<Arc<T>> = None;
            for priority in 0 .. self.prio_level_num {
                return_task = self.ready_queues[priority].lock().pop_front();
                if return_task.is_some() {
                    break;
                }
            }
            return_task
        }

        #[cfg(feature = "moic")]
        {

        }
    }

    pub fn remove(&mut self, task: Arc<T>) {
        #[cfg(not(feature = "moic"))]
        {
            for priority in 0 .. self.prio_level_num {
                self.ready_queues[priority].lock().retain(|task_in_queue| {
                    !Arc::ptr_eq(task_in_queue, &task)
                })
            }
        }

        #[cfg(feature = "moic")]
        {

        }
    }
}