use scheduler::{ CFScheduler, CFSTask, FifoScheduler, FifoTask, RRScheduler, RRTask };

mod static_priority;

cfg_if::cfg_if! {
    if #[cfg(feature = "sched_rr")] {
        const MAX_TIME_SLICE: usize = 5;
        pub type AxTask<T> = scheduler::RRTask<T, MAX_TIME_SLICE>;
        pub type Scheduler<T> = scheduler::RRScheduler<T, MAX_TIME_SLICE>;
    } else if #[cfg(feature = "sched_cfs")] {
        pub type AxTask<T> = scheduler::CFSTask<T>;
        pub type Scheduler<T> = scheduler::CFScheduler<T>;
    } else {
        // If no scheduler features are set, use FIFO as the default.
        pub type AxTask<T> = scheduler::FifoTask<T>;
        pub type Scheduler<T> = scheduler::FifoScheduler<T>;
    }
}