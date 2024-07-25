use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

pub struct BlockQueue<T> {
    queue: VecDeque<Arc<T>>
}

impl<T> BlockQueue<T> {
    pub fn new() -> BlockQueue<T> {
        Self {
            queue: VecDeque::new()
        }
    }

    pub fn add(&mut self, task: Arc<T>) {
        self.queue.push_back(task)
    }

    // wake_action的返回值：true代表中止遍历，false代表继续遍历。
    fn wake_raw_with_cond<F, G>(&mut self, cond: F, mut wake_action: G)
    where F: Fn(&T) -> bool, G: FnMut(Arc<T>) -> bool {
        for _ in 0 .. self.queue.len() {
            let task = self.queue.pop_front().unwrap();
            if cond(&task) {
                if wake_action(task) {
                    break;
                }
            }
            else {
                self.queue.push_back(task)
            }
        }
    }

    pub fn wake_one_with_cond<F>(&mut self, cond: F) -> Option<Arc<T>>
    where F: Fn(&T) -> bool {
        let mut return_task: Option<Arc<T>> = None;
        self.wake_raw_with_cond(cond, |task| {
            return_task = Some(task);
            true
        });
        return_task
    }

    pub fn wake_one(&mut self) -> Option<Arc<T>> {
        self.wake_one_with_cond(|_| { true })
    }

    pub fn wake_all_with_cond<F>(&mut self, cond: F) -> Vec<Arc<T>>
    where F: Fn(&T) -> bool {
        let mut ret_vec: Vec<Arc<T>> = Vec::new();
        self.wake_raw_with_cond(cond, |task| {
            ret_vec.push(task);
            false
        });
        ret_vec
    }

    pub fn wake_all(&mut self) -> Vec<Arc<T>> {
        self.wake_all_with_cond(|_| { true })
    }

    // /// 返回值代表唤醒任务的个数
    // pub fn wake_one_to_scheduler_with_cond<F>(&mut self, scheduler: &mut Scheduler<T>, cond: F) -> usize
    // where F: Fn(&T) -> bool {
    //     let mut wake_task_num: usize = 0;
    //     self.wake_raw_with_cond(cond, |task| {
    //         scheduler.add(task);
    //         wake_task_num += 1;
    //         true
    //     });
    //     wake_task_num
    // }

    // pub fn wake_one_to_scheduler(&mut self, scheduler: &mut Scheduler<T>) -> usize {
    //     self.wake_one_to_scheduler_with_cond(scheduler, |_| { true })
    // }

    // pub fn wake_all_to_scheduler_with_cond<F>(&mut self, scheduler: &mut Scheduler<T>, cond: F) -> usize
    // where F: Fn(&T) -> bool {
    //     let mut wake_task_num: usize = 0;
    //     self.wake_raw_with_cond(cond, |task| {
    //         scheduler.add(task);
    //         wake_task_num += 1;
    //         false
    //     });
    //     wake_task_num
    // }

    // pub fn wake_all_to_scheduler(&mut self, scheduler: &mut Scheduler<T>) -> usize {
    //     self.wake_all_to_scheduler_with_cond(scheduler, |_| { true })
    // }
}