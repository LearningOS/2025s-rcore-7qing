## 设计作业

### 关于之前的 syscall

​	由于我们引入了`Processor`因此，前面的逻辑也要有所变化，然后，我们虚拟地址转化物理地址也有包装好的`translated_byte_buffer`我们直接使用就可以，其余部分的逻辑跟之前并没有什么变化

### 进程创建

​	这里要求我们实在spawn进程，因此，我们可以根据将其类似为fork + exec ，其中，不必像 fork 一样复制父进程的地址空间。我们只需要新建一个地址空间，其中，我们需要新建一些虚拟区域，然后就可以完成，整体类似于与fork + exec的逻辑，其中，只是没有复制其地址空间。

### stride 调度算法

​	我们可以直接参考manager中的fifo的实现来进行更改，其中，manager增加BIG_STRIDE这个成员，然后tcb中的inner加入priority和Stride这两个成员（因为是可变的）。然后初始化的时候按照要求：

- 进程初始 stride 设置为 0 即可。
- 进程初始优先级设置为 16。

​	进行设置，然后add方法的实现与fifo的并没有什么区别，而fetch方法则需要有很大的改变，由于测例很简单，我们只需要整体遍历一遍就可以，寻找其中最小的Stride的。记录，然后将其从队列中取出，返回。

​	这里我们需要多设计一个pub函数add_stride,其作用是每次时间到的时候对其进行Stride加上pass。此后，我们便将add_stride加载切换进程的时候，最后，我们在完成sys_set_priority的设计，这个逻辑设计的十分简单，我们需要按照要求，并且更改curr_task的priority就可以。



## 问答作业

stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

- 实际情况是轮到 p1 执行吗？为什么？

不会，因为此时溢出了，在u8下250+10=4,4<255，根据我们的逻辑，此时仍然是p2执行任务

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， **在不考虑溢出的情况下** , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

- 为什么？尝试简单说明（不要求严格证明）。

  假设所有进程的初始 `stride` 都是大于等于 2 的值，即在 `stride` 初始化时，所有进程的优先级都足够高，不会过快地被调度。由于 `stride` 是线性递增的（每个时间片都会增加一个固定的值），因此，随着时间的推移，`stride` 增加的幅度相对较大。假设 `stride` 在一个很长的时间段内持续增加，进程的优先级变化不会过快。

- 已知以上结论，**考虑溢出的情况下**，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 `partial_cmp` 函数，假设两个 Stride 永远不会相等。

```
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_wrapped_around() && !other.is_wrapped_around() {
            return Some(Ordering::Greater); 
        }
        if !self.is_wrapped_around() && other.is_wrapped_around() {
            return Some(Ordering::Less); 
        }
        if self.0 < other.0 {
            return Some(Ordering::Less);
        } else {
            return Some(Ordering::Greater);
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: `(125 < 255) == false`, `(129 < 255) == true`.

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > *无*

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > *https://github.com/torvalds/linux/kernel/sched*
   >
   > https://learningos.cn/rCore-Tutorial-Guide-2025S/chapter4/3sv39-implementation-1.html

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。