use egui::{CentralPanel, Context};
use egui_plot::{Line, Plot, PlotPoints};
use pyo3::prelude::*;

mod run_native;

#[derive(Default)]
struct MyApp {
    xy_data: Vec<[f64; 2]>,
}

impl run_native::App for MyApp {
    fn update(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            Plot::new("A plot").show(ui, |pui| {
                pui.line(Line::new(PlotPoints::from_iter(
                    self.xy_data.clone().into_iter(),
                )))
            })
        });
    }
}

#[pymodule]
mod pyegui {
    use std::thread::{spawn, JoinHandle};

    use pyo3::types::PyList;

    use self::run_native::run_native;

    use super::*;

    #[pyclass]
    struct MyAppPyCtx {
        join_handle: JoinHandle<()>,
    }

    #[pymethods]
    impl MyAppPyCtx {
        #[new]
        fn new(x: Bound<'_, PyAny>, y: Bound<'_, PyAny>) -> PyResult<Self> {
            let x = x.downcast::<PyList>()?;
            let y = y.downcast::<PyList>()?;
            let xy_data = x
                .iter()
                .zip(y.iter())
                .filter_map(|(xf, yf)| match (xf.extract(), yf.extract()) {
                    (Ok(x), Ok(y)) => Some([x, y]),
                    _ => None,
                })
                .collect();
            let join_handle = spawn(|| run_native("Egui plot", Box::new(MyApp { xy_data })));
            Ok(Self { join_handle })
        }

        fn is_running(self_: PyRef<'_, Self>) -> bool {
            !self_.join_handle.is_finished()
        }
    }
}
