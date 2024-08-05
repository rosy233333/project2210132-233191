use alloc::sync::Arc;
use axlog::{debug, error, info};
use spinlock::SpinNoIrqOnlyGuard;
use core::{arch::asm, mem::ManuallyDrop, ops::Deref, task::Poll};
use crate::{processor::{self, Processor}, task::TaskState};

// use crate::{current_processor, processor::PrevCtxSave, stack_pool::TaskStack, AxTaskRef, CurrentTask, TaskState};
use super::{reg_context::{load_next_ctx, save_prev_ctx}, waker::waker_from_task, Task, TaskContext};

// #[cfg(feature = "preempt")]
/// This is only used when the preempt feature is enabled.
pub(crate) fn preempt_switch_entry(taskctx: &mut TaskContext) {
    let prev_task = Processor::with_current(|processor| {
        processor.acquire_switch_guard();
        processor.current_task().get_current_ptr()
    });

    // debug
    let id = prev_task.id();
    debug!("into preempt_switch_entry() with prev task {id}");

    prev_task.set_ctx_ref(taskctx as _);
    unsafe { schedule_with_sp_change(); }
}

/// This function is the entrance of activie switching.
pub(crate) fn switch_entry(is_thread: bool) {
    // 因为processor.acquire_switch_guard()会修改sstatus以禁止中断，因此将修改时保存的中断状态传入save_prev_ctx()，保存在TaskContext中。
    let (prev_task, s_irq_flag) = Processor::with_current(|processor| {
        processor.acquire_switch_guard();
        (processor.current_task().get_current_ptr(), processor.get_sstatus_in_switch_guard())
    });

    // debug
    let id = prev_task.id();
    debug!("into switch_entry() with prev task {id}");

    if is_thread {
        // // debug
        // error!("task {id} save context");
        unsafe { save_prev_ctx(&mut *prev_task.get_ctx_ref()); } // 该函数会调用schedule_with_sp_change()
    }
    else {
        schedule_without_sp_change();
    }
}

// 之前在rust代码中插入改变sp的内联汇编，发现在Release模式下会出错（Debug模式下不会）。
// 考虑到可能是因为rust代码编译出了在换栈后访问原栈上的数据的汇编指令，因此将整个函数改为汇编实现。
#[no_mangle]
#[naked]
pub(super) unsafe extern "C" fn schedule_with_sp_change() {
    asm!(
        "
        call    {before_change_stack}
        ",
        // Dangerous: it will change stack in the rust function, which can cause undefined behavior.
        "
        mv      sp, a0
        ",
        "
        jal     ra, {schedule_without_sp_change}
        j       -4
        ",
        before_change_stack = sym before_change_stack,
        schedule_without_sp_change = sym schedule_without_sp_change,
        options(noreturn),
    );
}

#[no_mangle]
fn before_change_stack() -> usize {
    Processor::with_current(|processor| {
        let new_stack = processor.get_stack_pool_mut().fetch();
        let new_stack_top = new_stack.top();
        let old_stack = processor.get_stack_pool_mut().swap_curr_stack(Some(new_stack));
        let prev_task = processor.current_task().get_current_ptr();

        // debug
        let id = prev_task.id();
        debug!("into schedule_with_sp_change() with prev task {id}");

        // 此时CPU的current_stack必须为Some，但除了从original task切换到其它任务以外（此时CPU使用的栈不被current_stack管理）。
        assert!(old_stack.is_some() || prev_task.is_original());
        // let prev_task = processor.current_task().get_current_ptr();
        assert!(prev_task.swap_owned_stack(old_stack).is_none());
        new_stack_top
    })
}

/// Pick next task from the scheduler and run it.
/// The prev task is a coroutine and the current stack will be reused.
#[no_mangle]
fn schedule_without_sp_change() {
    let next_task = Processor::with_current(|processor| {
        processor.pick_next_task()
    });

    // debug
    let id = next_task.id();
    debug!("into schedule_without_sp_change() with next task {id}");

    exchange_current(next_task);
}

/// Change the current status
/// 
/// Include the Processor and current task
fn exchange_current(mut next_task: Arc<Task>) {

    // debug
    let id = next_task.id();
    debug!("into exchange_current() with next task {id}");

    Processor::with_current(|processor| {
        let prev_task = processor.current_task().get_current_ptr();
        // // task in a disable_preempt context? it not allowed ctx switch
        // #[cfg(feature = "preempt")]
        // assert!(
        //     prev_task.can_preempt(),
        //     "task can_preempt failed {}",
        //     prev_task.id_name()
        // );
        // Here must lock curr state, and no one can change curr state
        // when excuting ctx_switch
        let mut prev_state_lock = prev_task.state_lock_manual();
        loop {
            match **prev_state_lock {
                TaskState::Runable => {
                    if next_task.is_idle() {
                        next_task = prev_task.clone();
                        break;
                    }
                    if !prev_task.is_idle() {
                        // #[cfg(feature = "preempt")]
                        // current_processor()
                        //     .put_prev_task(prev_task.clone(), prev_task.get_preempt_pending());
                        // #[cfg(not(feature = "preempt"))]
                        // current_processor().put_prev_task(prev_task.clone(), false);
                        processor.add_task_to_local(prev_task.clone());
                    }
                    break;
                }
                TaskState::Blocking => {
                    // debug!("task block: {}", prev_task.id_name());
                    **prev_state_lock = TaskState::Blocked;
                    break;
                }
                TaskState::Exited => {
                    break;
                }
                _ => {
                    panic!("unexpect state when switch_to happend ");
                }
            }
        }
        ManuallyDrop::into_inner(prev_state_lock);

        processor.current_task().replace_current(next_task);
    });


    // #[cfg(feature = "preempt")]
    // // reset preempt pending
    // next_task.set_preempt_pending(false);

    run_next();
}

/// Run next task
fn run_next() {
    // SAFETY: INIT when switch_to
    // First into task entry, manually perform the subsequent work of switch_to

    // 疑问：Processsor的PrevCtxSave和switch_post似乎没有作用？
    // current_processor().switch_post(); 

    let next_task = Processor::with_current(|processor| {
        processor.current_task().get_current_ptr()
    });

    // debug
    let id = next_task.id();
    debug!("into run_next() with next task {id}");

    if next_task.is_thread() {
        let task_ctx_ref = next_task.get_ctx_ref();
        // Dangerous: the current stack will be recycled. 
        // But it is used until executing the `load_next_ctx` function.
        Processor::with_current(|processor| {
            let new_stack = next_task.swap_owned_stack(None);
            assert!(new_stack.is_some());
            let old_stack = processor.get_stack_pool_mut().swap_curr_stack(new_stack);
            // original_task持有的栈不被processor数据结构管理
            assert!(old_stack.is_some() || next_task.is_original());
            if old_stack.is_some() {
                unsafe { processor.get_stack_pool_mut().recycle_stack(old_stack.unwrap()); }
            }
            processor.release_switch_guard();
        });
        unsafe {
            // // debug
            // error!("task {id} load context:");
            // error!("{:?}", *(*task_ctx_ref).as_ref());
            load_next_ctx(&mut *task_ctx_ref);
        }
    } else {
        let waker = waker_from_task(next_task.clone());
        let mut cx = core::task::Context::from_waker(&waker);
        let future = unsafe { &mut *next_task.get_future() };

        Processor::with_current(|processor| {
            processor.release_switch_guard();
        });
        assert!(future.as_mut().poll(&mut cx).is_pending());
        // 此处，任务执行后回到任务模块，因此需要再次获得switch_guard。
        Processor::with_current(|processor| {
            processor.acquire_switch_guard();
        });
    }
}
