## 设计作业

**linkat**：

​	要求我们去完成`pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize`这样的调用接口，仿照上面的sys_open()我们可以也写一个类似于open_file的函数，其形式为：`pub fn __sys_linkat(_old_name: &str, _new_name: &str) -> isize `我们在__sys_linkat中就可以调用ROOTINODE中的linkat方法。对于linkat方法，我们可以参照上面create方法，我们只需要注意以下：` pub fn linkat(&self, _old_name: &str, _new_name: &str) -> isize`

* 判断oldname的文件不为空，为空直接返回负一
* 由oldname中寻找inode_id,而不是像create中直接alloc_inode得到

**unlinkat**:

​	前面于linkat相同，当到了ROOTINODE中的unlink方法`pub fn unlink(&self, name: &str) -> isize`接下来，其思路和linkat完全相反，由于这里不要求我们删除，我们便不管

**fstat**:

​	参考文档，我们只需要获取ino和nlink。其中ino我们写一个osnode中的`get_inode_id`,其中，类型转化我们写一个any的trait，将其转换，nlink我们写一个get_link_num的方法，数有几个DirEntry。再将其数据写入_st中，我们便完成的fstat的写入

## 问答作业

​	root inode是根目录的 `Inode`，因为我们目前仅支持绝对路径，对于任何文件/目录的索引都必须从根目录开始向下逐级进行，起着定位的作用，如果root inode中的内容损坏了，那么我们将无法向下寻找目录，整个文件系统的任何文件/目录的索引都无法使用。

1. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > *https://github.com/torvalds/linux/fs*

2. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

3. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

