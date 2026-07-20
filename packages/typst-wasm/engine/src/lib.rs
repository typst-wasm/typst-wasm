mod compile;
mod dependencies;
mod diagnostics;
mod files;
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

    fn add_file(&self, path: String, data: Vec<u8>) -> Result<(), OperationError> {
        files::add_file(&mut self.state.borrow_mut(), path, data)
    }

    fn add_source(&self, path: String, text: String) -> Result<(), OperationError> {
        files::add_source(&mut self.state.borrow_mut(), path, text)
    }

    fn set_main(&self, path: String) -> Result<(), OperationError> {
        files::set_main(&mut self.state.borrow_mut(), path)
    }

    fn remove_file(&self, path: String) -> Result<bool, OperationError> {
        files::remove_file(&mut self.state.borrow_mut(), path)
    }

    fn clear_files(&self) -> Result<(), OperationError> {
        files::clear_files(&mut self.state.borrow_mut());
        Ok(())
    }

    fn list_files(&self) -> Result<Vec<String>, OperationError> {
        Ok(files::list_files(&self.state.borrow()))
    }

    fn has_file(&self, path: String) -> Result<bool, OperationError> {
        files::has_file(&self.state.borrow(), path)
    }

    fn compile(&self, options: CompileOptions) -> Result<CompileSuccess, CompileFailure> {
        compile::compile(&self.state, options)
    }
}

export!(Component);
