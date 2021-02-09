# Why is the 'a bound on T required? (lifetime)

Rust automatically enforces the bound T: 'a and will complain if type T lives shorter than the lifetime 'a. For instance, if T is &'b str and 'b is strictly shorter than 'a, Rust won’t allow you to create the instance of StackVec<'a, &'b str>.

Why is the bound required? What could go wrong if the bound wasn’t enforced by Rust?
