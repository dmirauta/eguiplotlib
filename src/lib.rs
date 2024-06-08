use egui::Context;
use egui_winit::egui::CentralPanel;
use pyo3::prelude::*;
use run_native::WinitGlowApp;

mod run_native;

#[derive(Default)]
struct MyApp {}

impl WinitGlowApp for MyApp {
    fn update(&mut self, ctx: &Context, quit: &mut bool) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello");
            if ui.button("close").clicked() {
                *quit = true;
            }
        });
    }
}

#[pymodule]
mod pyegui {
    use std::thread::{spawn, JoinHandle};

    use self::run_native::run_native;

    use super::*;

    #[pyclass]
    struct MyAppPyCtx {
        join_handle: JoinHandle<()>,
    }

    #[pymethods]
    impl MyAppPyCtx {
        #[new]
        fn new() -> Self {
            let join_handle = spawn(|| run_native(Box::new(MyApp {})));
            Self { join_handle }
        }

        fn is_running(self_: PyRef<'_, Self>) -> bool {
            !self_.join_handle.is_finished()
        }
    }
}
