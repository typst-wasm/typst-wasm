mod compile;
mod dependencies;
mod diagnostics;
mod fonts;
mod metadata;
mod paths;
mod state;
mod world;

mod export;
mod file_store;

use std::cell::RefCell;

wit_bindgen::generate!({
    path: "wit",
    world: "engine",
});

use exports::typst::engine::api::{
    CompileFailure, CompileOptions, CompileSuccess, Guest, GuestCompiler, OperationError,
};

use state::CompilerState;

struct Component;

pub struct Compiler {
    state: RefCell<CompilerState>,
}

impl Guest for Component {
    type Compiler = Compiler;
}

impl GuestCompiler for Compiler {
    fn new() -> Self {
        Self {
            state: RefCell::new(CompilerState::new()),
        }
    }

    fn add_font(&self, data: Vec<u8>) -> Result<String, OperationError> {
        fonts::add_font(&mut self.state.borrow_mut(), data)
    }

    fn compile(&self, options: CompileOptions) -> Result<CompileSuccess, CompileFailure> {
        compile::compile(&self.state, options)
    }
}

export!(Component);
