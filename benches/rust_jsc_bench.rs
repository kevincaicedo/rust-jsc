use criterion::criterion_main;

mod benchmarks {
    pub mod array;
    pub mod class;
    pub mod context;
    pub mod embedding;
    pub mod function;
    pub mod gc;
    pub mod inspector;
    pub mod module;
    pub mod object;
    pub mod promise;
    pub mod string;
    pub mod value;
}

criterion_main!(
    benchmarks::array::benches,
    benchmarks::class::benches,
    benchmarks::context::benches,
    benchmarks::embedding::benches,
    benchmarks::function::benches,
    benchmarks::gc::benches,
    benchmarks::inspector::benches,
    benchmarks::module::benches,
    benchmarks::object::benches,
    benchmarks::promise::benches,
    benchmarks::string::benches,
    benchmarks::value::benches,
);
