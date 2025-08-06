// Type your code here, or load an example.

// As of Rust 1.75, small functions are automatically
// marked as `#[inline]` so they will not show up in
// the output when compiling with optimisations. Use
// `#[no_mangle]` or `#[inline(never)]` to work around
// this issue.
// See https://github.com/compiler-explorer/compiler-explorer/issues/5939
// #[no_mangle]
fn double(mut num: A) -> A {
    num.val *= 2;
    let ptr = &num as *const A;
    println!("addr in sub process: {:p}", ptr);
    return num;
}

struct A {
    pub val: i32,
}

// If you use `main()`, declare it as `pub` to see it in the output:
fn main() {
    let a = A { val: 1 };
    let b = double(a);
    let ptr = &b as *const A;
    println!("addr in main process: {:p}", ptr);
    println!("{}", b.val)
}
