# 项目介绍文档

## 选择的赛题

proj68：异步操作系统中的进程、线程和协程调度

在RISC-V平台上利用Rust语言的异步特征，在操作系统内核中设计和实现统一的进程、线程和协程调度器，以提高操作系统整体性能。

## 项目描述

项目名称：通用任务调度模块

编写不与特定操作系统绑定、适用于多种基于Rust语言和RISC-V指令集的操作系统的任务调度模块。它可以直接作为此类系统的组件，从而方便此类系统的开发。该任务调度模块能统一进行进程、线程与协程的调度，统一各类任务的管理和切换方式。

本项目的独特之处有两点。其一是支持多种操作系统（条件仅为使用Rust语言和RISC-V指令集），其二是实现进程、线程、协程的统一调度。

该任务调度系统支持的其它功能为：多核调度、抢占、中断处理、任务间通信，等等。

## 目前已完成的功能

- 向操作系统提供描述任务的接口和使用调度器的接口
- 队列管理功能（就绪队列、阻塞队列）
- 任务运行功能
- 提供任务调度算法的接口，支持多种任务调度算法
- 支持多核
- 支持线程、协程的统一调度

## 之后计划完成的功能

- 中断处理与任务抢占，并将中断处理与任务调度结合
- 支持进程，实现多级调度
- 实现任务间通信机制

## 项目仓库结构

### doc目录

存放设计和开发过程中产生的各种文档。

### code目录

存储项目的代码。

项目的核心为`task_management`模块和`task_queues`模块。

`task_queues`模块实现了任务的队列管理功能。其中实现了调度器（及就绪队列）、阻塞队列、当前任务等存放任务的数据结构。该模块用于为`task_management`提供队列管理的支持，但不与`task_management`绑定，可以独立使用。

`task_management`模块实现了任务的数据结构和切换机制，并为外界提供了创建任务、让出、阻塞等任务管理API。。同时，还实现了代表处理器的数据结构、代表栈的数据结构等，从而支持任务的调度和运行。

`kernel_guard`和`scheduler`模块为引用已有的模块并加以修改。而`dependencies`中的模块是被我们的项目依赖，并依赖我们修改后的模块的那些模块。因此修改了它们的依赖路径，并未进行其它修改。

### Starry目录

该目录是将本项目部署在Starry系统上运行和演示的代码。

## 运行和演示

### 介绍

本项目目前部署在Starry系统上进行演示，使用Starry系统的目的是提供内存分配功能与输出功能。

演示中，使用了Starry系统的unikernel模式。在该模式下，用户程序和系统内核均运行在内核态，没有地址空间的隔离。我们将项目放在用户程序中运行，并修改了Starry系统的启动代码，使得用户程序可以使用每个CPU核心。

Starry的模块化特性，使得可以对它进行配置，自由地开启和关闭一些功能。在测试程序目录`Starry/apps/scheduler_test`的`Cargo.toml`中，对`axstd`的依赖没有启用`multitask`功能，说明测试过程没有使用Starry本身的多任务功能，而是由本项目提供多任务的支持。

在我们编写的测试程序中，主线程会创建多个线程和协程，每个被创建的线程和协程都会进行让出、阻塞、唤醒等操作。观察它们同时执行时的行为，从而验证任务调度器的正确性。

### 运行方法

首先，确保项目的所有子模块（submodule）均被下载到本地。

之后，在项目根目录下，执行下列操作：

```Bash
$ cd Starry
$ ./build_img.sh -a riscv64
$ make A=apps/scheduler_test LOG=warn ARCH=riscv64 SMP=4 run
```

### 演示现象

![](doc/assets/屏幕截图%202024-07-31%20173942.png)

执行结果如图所示。

从图中可以发现，线程和协程可以在不同的CPU核心上运行，且在让出和阻塞时，在同一核心上交错执行。

让出操作不会改变任务所在的CPU核心，而阻塞-唤醒操作可以。实际上，唤醒操作有唤醒到当前核心和唤醒到全局调度器两种版本，而让出操作目前仅支持放回当前CPU核心，之后也会增加放到全局调度器的版本。

[演示视频](doc/演示视频.mp4)

## 引用的其它作品

### [Starry操作系统](https://github.com/Starry-OS/Starry)

Starry系统基于[ArceOS系统](https://github.com/arceos-org/arceos)，是一个基于Rust语言的模块化操作系统，它将系统功能分割为许多模块，可以根据需要启用不同的模块组合，以为系统提供不同功能。

本项目参考了Starry中任务调度模块中，对任务状态的设计。也引用了一部分相对独立的模块，以实现本项目的非核心功能，例如提供关中断临界区的`kernel_guard`，以及提供调度算法的统一接口的`scheduler`（虽然`scheduler`模块里已有一些调度算法的实现，但本项目采用的优先级调度算法是自己实现的，只是使用了`scheduler`中的接口）。

### 赵方亮的工作

赵方亮学长当前的研究工作也是结合异步的任务调度机制。他的工作之一是[AsyncStarry系统](https://github.com/zflcs/AsyncStarry/tree/dev)。在该系统中，赵方亮修改了Starry系统，设计了一个统一了线程和协程的上下文存储机制和切换机制，并为Starry系统添加了协程支持。本项目使用了该机制。但他的实现与Starry系统绑定，而本项目实现的切换机制可以应用于不同的系统中。

同时，赵方亮的工作基于他开发的自定义硬件`MOIC`，而本项目的实现不需要特殊的硬件。