# moic_driver调研

时间：2024/6/29

## 学长的 `moic_driver` 的功能

已有的功能：

- 创建任务 `TaskMeta` ，将其与 `TaskId` 互相转化。
- 向 `Moic` 中加入任务、取出任务、移除任务
- 在 `Moic` 中切换 hypervisor 、 os 、 process 。
- 注册发送方和接收方、发送中断（包括IPC和外部中断）

## 兼容 MOIC 时需要考虑的内容：

- **中断与IPC：** 注册中断处理程序/注册IPC发方收方，我们的框架需要支撑该功能吗？（我们的系统中，可以作为一种特殊的阻塞队列？）
  - 同时，需要向学长询问 MOIC 中的注册的更多细节，例如：中断处理程序/ IPC 发方是以进程、线程还是协程为单位注册的？线程或协程
- **多级调度：** hypervisor 、 os 、 process 、 thread 、 coroutine ，在 MOIC 中是如何进行多级调度的，而我的系统又要如何实现？
  - os 属于 hypervisor 
  - （用户）process 、 （内核）thread 、 （内核）coroutine 属于 os 
  - （用户）thread 、 （用户）coroutine 属于 （用户）process
  - 尚不知道 thread 和 coroutine 的关系