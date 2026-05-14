use rust_jsc::{module_fetcher, JSContext, ModuleImportType};

struct NotModuleSource;

#[module_fetcher]
fn fetch(
    _ctx: JSContext,
    _key: String,
    _import_type: ModuleImportType,
) -> NotModuleSource {
    NotModuleSource
}

fn main() {}
