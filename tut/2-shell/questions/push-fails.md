The push method from Vec in the standard library has no return value, but the push method from our StackVec does: it returns a Result indicating that it can fail. Why can StackVec::push() fail where Vec::push() does not?

`StackVec`使用静态区存储，因此长度在编译期就固定。`StackVec::push()`不能无限次调用，当要压入栈的长度大于可接受的长度，就需要返回`Err`。相反，`Vec`在堆上存储，因而理论上能够无限次数`push`。
