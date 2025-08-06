#![allow(unused)]
fn print_link(link: String, Method(method): Method) {
    println!("HTTP: {method} {link}");
}

fn print_all(link: String, Method(method): Method, Args(args): Args) {
    println!("HTTP: {method} {link}\nARGS: {args:?}");
}

fn print_rev(link: String, Args(args): Args, Method(method): Method) {
    println!("HTTP: {method} {link}\nARGS: {args:?}");
}

#[derive(Clone)]
pub struct RequestResolver {
    link: String,
    method: &'static str,
    args: Vec<String>,
}
// 这个部分演示需要, 实际不需要, 框架里是外部会传递整个 Context
impl RequestResolver {
    pub fn new(link: &str) -> Self {
        Self {
            link: link.to_string(),
            method: "GET",
            args: vec![],
        }
    }
    pub fn with_args(mut self, args: &[&'static str]) -> Self {
        self.args = args.iter().map(|s| s.to_string()).collect();
        self
    }
}

pub struct Method(pub &'static str);
pub struct Args(pub Vec<String>);

pub trait FromContext {
    type Context;
    fn from_context(context: &Self::Context) -> Self;
}

impl FromContext for Args {
    type Context = RequestResolver;

    fn from_context(context: &Self::Context) -> Self {
        Args(context.args.clone())
    }
}

impl FromContext for Method {
    type Context = RequestResolver;

    fn from_context(context: &RequestResolver) -> Self {
        Method(context.method)
    }
}

impl FromContext for String {
    type Context = RequestResolver;

    fn from_context(context: &RequestResolver) -> Self {
        context.link.clone()
    }
}

pub trait Handler<T, C> {
    fn apply(self, context: &C);
}

impl<C, F, T1> Handler<T1, C> for F
where
    F: Fn(T1),
    T1: FromContext<Context = C>,
{
    fn apply(self, context: &C) {
        (self)(T1::from_context(context));
    }
}

impl<C, F, T1, T2> Handler<(T1, T2), C> for F
where
    F: Fn(T1, T2),
    T1: FromContext<Context = C>,
    T2: FromContext<Context = C>,
{
    fn apply(self, context: &C) {
        (self)(T1::from_context(context), T2::from_context(context));
    }
}

impl<
    C,
    F,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10,
    T11,
    T12,
    T13,
    T14,
    T15,
    T16,
    T17,
    T18,
    T19,
    T20,
    T21,
    T22,
>
    Handler<
        (
            T1,
            T2,
            T3,
            T4,
            T5,
            T6,
            T7,
            T8,
            T9,
            T10,
            T11,
            T12,
            T13,
            T14,
            T15,
            T16,
            T17,
            T18,
            T19,
            T20,
            T21,
            T22,
        ),
        C,
    > for F
where
    F: Fn(
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16,
        T17,
        T18,
        T19,
        T20,
        T21,
        T22,
    ),
    T1: FromContext<Context = C>,
    T2: FromContext<Context = C>,
    T3: FromContext<Context = C>,
    T4: FromContext<Context = C>,
    T5: FromContext<Context = C>,
    T6: FromContext<Context = C>,
    T7: FromContext<Context = C>,
    T8: FromContext<Context = C>,
    T9: FromContext<Context = C>,
    T10: FromContext<Context = C>,
    T11: FromContext<Context = C>,
    T12: FromContext<Context = C>,
    T13: FromContext<Context = C>,
    T14: FromContext<Context = C>,
    T15: FromContext<Context = C>,
    T16: FromContext<Context = C>,
    T17: FromContext<Context = C>,
    T18: FromContext<Context = C>,
    T19: FromContext<Context = C>,
    T20: FromContext<Context = C>,
    T21: FromContext<Context = C>,
    T22: FromContext<Context = C>,
{
    fn apply(self, context: &C) {
        (self)(
            T1::from_context(context),
            T2::from_context(context),
            T3::from_context(context),
            T4::from_context(context),
            T5::from_context(context),
            T6::from_context(context),
            T7::from_context(context),
            T8::from_context(context),
            T9::from_context(context),
            T10::from_context(context),
            T11::from_context(context),
            T12::from_context(context),
            T13::from_context(context),
            T14::from_context(context),
            T15::from_context(context),
            T16::from_context(context),
            T17::from_context(context),
            T18::from_context(context),
            T19::from_context(context),
            T20::from_context(context),
            T21::from_context(context),
            T22::from_context(context),
        );
    }
}

fn main() {
    let a = Args(vec!["11".into(), "2".into()]);
    let m = Method("get");
    print_all("hh".into(), m, a);
}
