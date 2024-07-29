use core::{future::{poll_fn, Future}, pin::Pin, task::{Context, Poll}};
use alloc::sync::Arc;

// ------处理器初始化------

/// 需要在主处理器上调用，且仅调用一次。
/// 这两个函数会初始化函数运行的处理器，然后开始运行主任务。
/// 从此之后，该cpu的执行流纳入cpu所属调度器的管理中。
/// 传入cpu_id和cpu_num是初始化per_cpu库的要求。
/// TODO: 暂未考虑main_task执行完成后，系统停止的问题，因此该函数不会返回。
pub fn init_main_processor<F>(main_task_fn: F, cpu_id: usize, cpu_num: usize) -> !
where F: (FnOnce() -> i32) + Send + 'static {
    Processor::init_main_processor(cpu_id, cpu_num);
    let main_task = TaskInner::new_init(main_task_fn);
    Processor::with_current(|processor| {
        processor.add_task_to_local(main_task);
        let current_task = processor.current_task().get_current_ptr();
        assert!(current_task.is_original());
        // 使得现有执行流不会加入调度器
        current_task.set_state(TaskState::Blocking);
    });

    // 开始从调度器中取出任务运行。
    switch_entry(true);

    // unreachable
    loop { }
}

#[cfg(feature = "smp")]
pub fn init_main_processor_with_async<F>(main_task_fn: F, cpu_id: usize, cpu_num: usize) -> !
where F: Future<Output = i32> + Send + 'static {
    Processor::init_main_processor(cpu_id, cpu_num);
    let main_task = TaskInner::new_async_init(main_task_fn);
    Processor::with_current(|processor| {
        processor.add_task_to_local(main_task);
        let current_task = processor.current_task().get_current_ptr();
        assert!(current_task.is_original());
        // 使得现有执行流不会加入调度器
        current_task.set_state(TaskState::Blocking);
    });

    // 开始从调度器中取出任务运行。
    switch_entry(true);

    // unreachable
    loop { }
}

/// 需要在副处理器上调用，且每个副处理器调用一次。
/// 需要在init_main_processor执行完成后调用。
/// 该函数也会初始化处理器，但在向调度器放入任务前，会运行处理器自带的“idle_task”
/// 从此之后，该cpu的执行流纳入cpu所属调度器的管理中。
pub fn init_secondary_processor(cpu_id: usize) -> ! {
    Processor::init_secondary_processor(cpu_id);
    Processor::with_current(|processor| {
        let current_task = processor.current_task().get_current_ptr();
        assert!(current_task.is_original());
        // 使得现有执行流不会加入调度器
        current_task.set_state(TaskState::Blocking);
    });

    // 开始从调度器中取出任务运行。
    switch_entry(true);

    // unreachable
    loop { }
}

// ------任务创建------

use spinlock::SpinNoIrq;
use crate::{processor::{self, Processor}, task::{preempt_switch_entry, switch_entry, TaskContext, TaskInner, TaskState}};
pub use crate::task::Task;

/// 创建任务并加入全局的调度器
pub fn spawn_to_global<F>(f: F) -> Arc<Task>
where F: (FnOnce() -> i32) + Send + 'static {
    let task = TaskInner::new(f);
    Processor::with_current(|processor| {
        processor.add_task_to_global(task.clone());
    });
    task
}
pub fn spawn_to_global_async<F>(f: F) -> Arc<Task>
where F: Future<Output = i32> + Send + 'static {
    let task = TaskInner::new_async(f);
    Processor::with_current(|processor| {
        processor.add_task_to_global(task.clone());
    });
    task
}

/// 创建任务并加入当前CPU的调度器
pub fn spawn_to_local<F>(f: F) -> Arc<Task>
where F: (FnOnce() -> i32) + Send + 'static {
    let task = TaskInner::new(f);
    Processor::with_current(|processor| {
        processor.add_task_to_local(task.clone());
    });
    task
}
pub fn spawn_to_local_async<F>(f: F) -> Arc<Task>
where F: Future<Output = i32> + Send + 'static {
    let task = TaskInner::new_async(f);
    Processor::with_current(|processor| {
        processor.add_task_to_local(task.clone());
    });
    task
}

// ------当前任务管理------

/// 获取当前任务的Arc实例
/// 向外部暴露的`Task`对象，功能尽可能少，从而保证大部分任务管理功能可以仅使用“当前任务管理”的接口完成。
/// 目前，获得的`Task`对象只有用于join这一个用途。
pub fn current_ptr() -> Arc<Task> {
    Processor::with_current(|processor| {
        processor.current_task().get_current_ptr()
    })
}

/// 改变当前任务的优先级
/// 返回值代表传入的优先级是否合法、修改是否成功
pub fn change_current_priority(new_priority: isize) -> bool {
    Processor::with_current(|processor| {
        let current = processor.current_task().get_current_ptr();
        processor.with_local_scheduler(|scheduler| {
            scheduler.set_priority(&current, new_priority)
        })
    })
}

/// 主动让权一次，且将任务放回当前CPU的调度器
pub fn yield_current_to_local() {
    Processor::with_current(|processor| {
        let current = processor.current_task().get_current_ptr();
        assert!(current.is_runable());
    });
    switch_entry(true);
}
pub async fn yield_current_to_local_async() {
    yield_helper().await;
}

// 暂时不考虑，因为可能出现同步问题：放入调度器后，还未保存上下文，就被其它CPU核心取出执行。
// 原因：目前运行和就绪状态都用TaskState::Runable表示。
// 且即使将运行和就绪状态区分开，调度器也没有“仅选取就绪态任务”的接口。
// /// 让权，且将任务放回全局调度器（可能被其它CPU核心执行）
// pub fn yield_current_to_global() {
//     unimplemented!()
// }
// pub async fn yield_current_to_global_async() {
//     unimplemented!()
// }

/// 用于使协程让出一次，切换到其它任务、
/// 功能相当于线程的switch_entry()
async fn yield_helper() {
    let mut flag = false;
    poll_fn(|_cx| {
        flag = !flag;
        if flag {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }).await;
}

/// 阻塞在阻塞队列中
pub fn block_current(block_queue: &mut BlockQueue) {
    block_queue.block_current()
}
pub async fn block_current_async(block_queue: &mut BlockQueue) {
    block_queue.block_current_async().await
}

// 目前先不考虑该接口，因为其涉及到时间与中断
// /// 睡眠
// pub fn sleep_current(duration: Duration)
// pub async fn sleep_current(duration: Duration)

/// 退出任务，可用于函数执行完毕的正常退出或中途退出
pub fn exit_current(exit_code: i32) {
    Processor::with_current(|processor| {
        let current = processor.current_task().get_current_ptr();
        // current_state作用域
        {
            let mut current_state = current.state_lock();
            assert!(matches!(*current_state, TaskState::Runable));
            current.set_exit_code(exit_code);
            *current_state = TaskState::Exited; // 状态为Exited的任务一定已经保存好了返回值
        }
    });
    switch_entry(true);
}
pub async fn exit_current_async(exit_code: i32) {
    Processor::with_current(|processor| {
        let current = processor.current_task().get_current_ptr();
        // current_state作用域
        {
            let mut current_state = current.state_lock();
            assert!(matches!(*current_state, TaskState::Runable));
            current.set_exit_code(exit_code);
            *current_state = TaskState::Exited; // 状态为Exited的任务一定已经保存好了返回值
        }
    });
    yield_helper().await;
}

// 目前先不考虑该接口
// /// 等待另一任务完成，并接收其返回值
// /// 设想是，在join时，使等待任务获取被等待任务的Arc实例，等到获取了该任务的返回值再释放该实例。
// pub fn current_join_another(task: Arc<Task>) -> i32
// pub async fn current_join_another_async(task: Arc<Task>)

/// 抢占当前任务
/// 传入的参数为中断时保存的Trap上下文，之后会将其作为任务上下文保存，这样恢复时可以直接恢复到任务中。
/// 被抢占的任务只能放回当前CPU的局部调度器。
/// 目前，该接口仅为中断处理函数准备。
pub fn premmpt_current(task_ctx: &mut TaskContext) {
    Processor::with_current(|processor| {
        let current = processor.current_task().get_current_ptr();
        assert!(current.is_runable());
    });
    preempt_switch_entry(task_ctx);
}

// ------阻塞队列的结构及管理------

/// 在任务调度/队列管理模块中，BlockQueue可以配合各种满足trait的任务数据结构；但在向用户暴露的接口中，BlockQueue仅配合Task使用。
use task_queues::{block_queue, scheduler::{self, BaseScheduler}};
pub struct BlockQueue(block_queue::BlockQueue<Task>);

impl BlockQueue {

    /// 创建阻塞队列
    /// 不知是否要考虑，用户拿到阻塞队列后，在不正确的时机drop掉，导致其中的任务也被drop掉的问题？
    pub fn new() -> Self {
        Self {
            0: block_queue::BlockQueue::new()
        }
    }

    /// 创建一个提供了多线程访问和内部可变性的阻塞队列
    pub fn new_arc() -> Arc<SpinNoIrq<Self>> {
        Arc::new(SpinNoIrq::new(Self {
            0: block_queue::BlockQueue::new()
        }))
    }

    /// 将当前任务阻塞在该队列上
    /// 与“当前任务管理”中的同名函数功能重复了，不知道要保留哪个，还是全部保留？
    pub fn block_current(&mut self) {
        Processor::with_current(|processor| {
            let current = processor.current_task().get_current_ptr();
            // current_state作用域
            {
                let mut current_state = current.state_lock();
                assert!(matches!(*current_state, TaskState::Runable));
                *current_state = TaskState::Blocking; // 状态为Exited的任务一定已经保存好了返回值
            }
            self.0.add(current);
        });
        switch_entry(true);
    }

    /// 将当前任务阻塞在该队列上
    /// 与“当前任务管理”中的同名函数功能重复了，不知道要保留哪个，还是全部保留？
    pub async fn block_current_async(&mut self) {
        Processor::with_current(|processor| {
            let current = processor.current_task().get_current_ptr();
            // current_state作用域
            {
                let mut current_state = current.state_lock();
                assert!(matches!(*current_state, TaskState::Runable));
                *current_state = TaskState::Blocking; // 状态为Exited的任务一定已经保存好了返回值
            }
            self.0.add(current);
        });
        yield_helper().await;
    }

    /// 从队列中唤醒任务，放入当前CPU核心的调度器中
    /// 根据唤醒的是一个任务还是多个、是否按条件唤醒（条件为真才会唤醒）、唤醒后加入当前CPU调度器还是全局调度器，具有八个版本
    /// 返回值代表实际唤醒的任务的数量
    pub fn wake_one_to_local(&mut self) -> usize {
        let task_option = self.0.wake_one_with_cond(|task| {
            task.is_blocked()
        });
        if let Some(task) = task_option {
            task.set_state(TaskState::Runable);
            Processor::with_current(|processor| {
                processor.add_task_to_local(task)
            });
            1
        }
        else {
            0
        }
    }
    pub fn wake_all_to_local(&mut self) -> usize {
        let tasks = self.0.wake_all_with_cond(|task| {
            task.is_blocked()
        });
        let task_num = tasks.len();
        if task_num != 0 {
            for task in &tasks {
                task.set_state(TaskState::Runable);
            };
            Processor::with_current(|processor| {
                for task in tasks {
                    processor.add_task_to_local(task)
                }
            });
        }
        task_num
    }
    pub fn wake_one_with_cond_to_local<F>(&mut self, cond: F) -> usize
    where F: Fn(&Task) -> bool {
        let task_option = self.0.wake_one_with_cond(|task| {
            cond(task) && task.is_blocked()
        });
        if let Some(task) = task_option {
            task.set_state(TaskState::Runable);
            Processor::with_current(|processor| {
                processor.add_task_to_local(task)
            });
            1
        }
        else {
            0
        }
    }
    pub fn wake_all_with_cond_to_local<F>(&mut self, cond: F) -> usize
    where F: Fn(&Task) -> bool {
        let tasks = self.0.wake_all_with_cond(|task| {
            cond(task) && task.is_blocked()
        });
        let task_num = tasks.len();
        if task_num != 0 {
            for task in &tasks {
                task.set_state(TaskState::Runable);
            };
            Processor::with_current(|processor| {
                for task in tasks {
                    processor.add_task_to_local(task)
                }
            });
        }
        task_num
    }

    pub fn wake_one_to_global(&mut self) -> usize {
        let task_option = self.0.wake_one_with_cond(|task| {
            task.is_blocked()
        });
        if let Some(task) = task_option {
            task.set_state(TaskState::Runable);
            Processor::with_current(|processor| {
                processor.add_task_to_global(task)
            });
            1
        }
        else {
            0
        }
    }
    pub fn wake_all_to_global(&mut self) -> usize {
        let tasks = self.0.wake_all_with_cond(|task| {
            task.is_blocked()
        });
        let task_num = tasks.len();
        if task_num != 0 {
            for task in &tasks {
                task.set_state(TaskState::Runable);
            };
            Processor::with_current(|processor| {
                for task in tasks {
                    processor.add_task_to_global(task)
                }
            });
        }
        task_num
    }
    pub fn wake_one_with_cond_to_global<F>(&mut self, cond: F) -> usize
    where F: Fn(&Task) -> bool {
        let task_option = self.0.wake_one_with_cond(|task| {
            cond(task) && task.is_blocked()
        });
        if let Some(task) = task_option {
            task.set_state(TaskState::Runable);
            Processor::with_current(|processor| {
                processor.add_task_to_global(task)
            });
            1
        }
        else {
            0
        }
    }
    pub fn wake_all_with_cond_to_global<F>(&mut self, cond: F) -> usize
    where F: Fn(&Task) -> bool {
        let tasks = self.0.wake_all_with_cond(|task| {
            cond(task) && task.is_blocked()
        });
        let task_num = tasks.len();
        if task_num != 0 {
            for task in &tasks {
                task.set_state(TaskState::Runable);
            };
            Processor::with_current(|processor| {
                for task in tasks {
                    processor.add_task_to_global(task)
                }
            });
        }
        task_num
    }
}