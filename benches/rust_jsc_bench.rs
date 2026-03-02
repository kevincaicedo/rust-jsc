use criterion::criterion_main;

mod benchmarks {
    pub mod array;
    pub mod context;
    pub mod function;
    pub mod gc;
    pub mod object;
    pub mod string;
    pub mod value;
}

criterion_main!(
    benchmarks::array::benches,
    benchmarks::context::benches,
    benchmarks::function::benches,
    benchmarks::gc::benches,
    benchmarks::object::benches,
    benchmarks::string::benches,
    benchmarks::value::benches,
);
