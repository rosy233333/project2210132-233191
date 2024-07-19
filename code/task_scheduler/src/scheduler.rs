#[cfg(not(feature = "moic"))]
use alloc::collections::VecDeque;
use alloc::sync::Arc;
#[cfg(not(feature = "moic"))]
use spinlock::SpinNoIrq;

pub struct Scheduler<T> {
    #[cfg(not(feature = "moic"))]
    ready_queue: SpinNoIrq<VecDeque<Arc<T>>>,
}

impl<T> Scheduler<T> {
    pub fn new() -> Scheduler<T> {
        Self {
            #[cfg(not(feature = "moic"))]
            ready_queue: SpinNoIrq::new(VecDeque::new())
        }
    }

    pub fn add(&mut self, task: Arc<T>) {
        #[cfg(not(feature = "moic"))]
        {
            self.ready_queue.lock().push_back(task);
        }

        #[cfg(feature = "moic")]
        {

        }
    }

    pub fn fetch(&mut self) -> Option<Arc<T>> {
        #[cfg(not(feature = "moic"))]
        {
            self.ready_queue.lock().pop_front()
        }

        #[cfg(feature = "moic")]
        {

        }
    }
}

impl<T> Scheduler<T>
{
    // 实现移除功能需要代表任务的类型可以判断是否相等。
    // 如果代表任务的类型是一个给定类型的指针，则可以将“相等”定义成它们指向相同的内存区域。
    pub fn remove(&mut self, task: Arc<T>) {
        #[cfg(not(feature = "moic"))]
        {
            self.ready_queue.lock().retain(|task_in_queue| {
                !Arc::ptr_eq(task_in_queue, &task)
            })
        }

        #[cfg(feature = "moic")]
        {

        }
    }
}