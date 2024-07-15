use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use crate::scheduler::Scheduler;

pub struct BlockQueue<T> {
    queue: VecDeque<(T, Box<dyn Fn(&T) -> bool>)>
}

impl<T> BlockQueue<T> {
    pub fn new() -> BlockQueue<T> {
        Self {
            queue: VecDeque::new()
        }
    }

    pub fn add(&mut self, task: T) {
        self.queue.push_back((task, Box::new(|_| { true })))
    }

    // 以这种方式加入的任务，在BlockQueue调用wake系列方法时，会先检查对应的cond结果，为真则唤醒，为假则保留在阻塞队列中。
    // 此外，如果调用该函数时，cond已经为真，则不会阻塞。
    pub fn add_with_cond<F>(&mut self, task: T, cond: F)
    where F: Fn(&T) -> bool + 'static {
        if !cond(&task) {
            self.queue.push_back((task, Box::new(cond)))
        } 
    }

    pub fn wake_one(&mut self) -> Option<T> {
        for _ in 0 .. self.queue.len() {
            let (task, cond) = self.queue.pop_front().unwrap();
            if cond(&task) {
                return Some(task);
            }
            else {
                self.queue.push_back((task, cond))
            }
        }
        None
    }

    pub fn wake_all(&mut self) -> Vec<T> {
        let mut ret_vec: Vec<T> = Vec::new();
        for _ in 0 .. self.queue.len() {
            let (task, cond) = self.queue.pop_front().unwrap();
            if cond(&task) {
                ret_vec.push(task);
            }
            else {
                self.queue.push_back((task, cond))
            }
        }
        ret_vec
    }

    pub fn wake_one_to_scheduler(&mut self, scheduler: &mut Scheduler<T>) -> usize {
        for _ in 0 .. self.queue.len() {
            let (task, cond) = self.queue.pop_front().unwrap();
            if cond(&task) {
                scheduler.add(task);
                return 1;
            }
            else {
                self.queue.push_back((task, cond))
            }
        }
        0
    }

    pub fn wake_all_to_scheduler(&mut self, scheduler: &mut Scheduler<T>) -> usize {
        let mut wake_num: usize = 0;
        for _ in 0 .. self.queue.len() {
            let (task, cond) = self.queue.pop_front().unwrap();
            if cond(&task) {
                scheduler.add(task);
                wake_num += 1;
            }
            else {
                self.queue.push_back((task, cond))
            }
        }
        wake_num
    }
}