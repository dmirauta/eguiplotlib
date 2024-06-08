use egui::{CentralPanel, Context, ScrollArea};
use egui_inspect::EguiInspect;
use egui_plot::{Line, PlotPoints};
use pyo3::{
    exceptions::PyIndexError,
    prelude::*,
    types::{PyInt, PyList},
};
use std::{
    sync::{Arc, Mutex},
    thread::{spawn, JoinHandle},
};

mod run_native;

type LineData = Vec<[f64; 2]>;

#[derive(Default, Clone)]
struct Plot {
    line_data: Vec<LineData>,
}

impl EguiInspect for Plot {
    fn inspect(&self, label: &str, ui: &mut egui::Ui) {
        egui_plot::Plot::new(label).show(ui, |pui| {
            for data in self.line_data.iter() {
                pui.line(Line::new(PlotPoints::from_iter(data.clone().into_iter())))
            }
        });
    }

    fn inspect_mut(&mut self, label: &str, ui: &mut egui::Ui) {
        self.inspect(label, ui);
    }
}

impl Plot {
    fn _set_line(&mut self, i: usize, x: Bound<'_, PyAny>, y: Bound<'_, PyAny>) -> PyResult<()> {
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
        let n = self.line_data.len();
        match i > n {
            true => Err(PyErr::new::<PyIndexError, _>(format!(
                "Idx {i} invalid for plot with {n} lines.",
            ))),
            false => {
                if i == n {
                    self.line_data.push(xy_data);
                } else {
                    self.line_data[i] = xy_data;
                }
                Ok(())
            }
        }
    }

    #[allow(dead_code)]
    fn set_line(
        &mut self,
        i: Bound<'_, PyAny>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let i: usize = i.downcast::<PyInt>()?.extract()?;
        self._set_line(i, x, y)
    }

    fn add_line(&mut self, x: Bound<'_, PyAny>, y: Bound<'_, PyAny>) -> PyResult<()> {
        self._set_line(self.line_data.len(), x, y)
    }
}

#[derive(Clone)]
struct PlotRow {
    plots: Vec<Plot>,
    height: f32,
}

impl EguiInspect for PlotRow {
    fn inspect(&self, label: &str, ui: &mut egui::Ui) {
        ui.columns(self.plots.len(), |columns| {
            for (i, (plot, col)) in self.plots.iter().zip(columns.iter_mut()).enumerate() {
                col.set_height(self.height);
                plot.inspect(format!("{label}[{i}]").as_str(), col);
            }
        })
    }

    fn inspect_mut(&mut self, label: &str, ui: &mut egui::Ui) {
        self.inspect(label, ui);
    }
}

struct Figure {
    plot_rows: Vec<PlotRow>,
}

impl Figure {
    fn new(n: usize, m: usize) -> Self {
        Self {
            plot_rows: vec![
                PlotRow {
                    plots: vec![Default::default(); m],
                    height: 100.0
                };
                n
            ],
        }
    }

    fn split_height(&mut self, height: f32) {
        let row_height = height / (self.plot_rows.len() as f32);
        for plot in self.plot_rows.iter_mut() {
            plot.height = row_height;
        }
    }
}

impl EguiInspect for Figure {
    fn inspect(&self, label: &str, ui: &mut egui::Ui) {
        for (i, pl) in self.plot_rows.iter().enumerate() {
            pl.inspect(format!("{label}[{i}]").as_str(), ui);
        }
    }

    fn inspect_mut(&mut self, label: &str, ui: &mut egui::Ui) {
        self.inspect(label, ui);
    }
}

struct PlotWindow {
    fig: Arc<Mutex<Figure>>,
}

impl run_native::App for PlotWindow {
    fn update(&mut self, ctx: &Context) {
        if let Ok(mut fig) = self.fig.try_lock() {
            CentralPanel::default().show(ctx, |ui| {
                let r = ScrollArea::both().show(ui, |ui| {
                    fig.inspect_mut("", ui);
                });
                fig.split_height(r.inner_rect.height());
            });
        }
    }
}

#[pymodule]
mod pyegui {

    use self::run_native::run_native;

    use super::*;

    #[pyclass]
    struct EguiFigure {
        join_handle: JoinHandle<()>,
        fig: Arc<Mutex<Figure>>,
    }

    #[pymethods]
    impl EguiFigure {
        #[new]
        fn new(n: Bound<'_, PyAny>, m: Bound<'_, PyAny>) -> PyResult<Self> {
            let n: usize = n.downcast::<PyInt>()?.extract()?;
            let m: usize = m.downcast::<PyInt>()?.extract()?;
            let fig = Arc::new(Mutex::new(Figure::new(n, m)));
            let _fig = fig.clone();
            let join_handle =
                spawn(move || run_native("Egui figure", Box::new(PlotWindow { fig: _fig })));
            Ok(Self { join_handle, fig })
        }

        fn is_running(self_: PyRef<'_, Self>) -> bool {
            !self_.join_handle.is_finished()
        }

        fn add_line(
            self_: PyRef<'_, Self>,
            i: Bound<'_, PyAny>,
            j: Bound<'_, PyAny>,
            x: Bound<'_, PyAny>,
            y: Bound<'_, PyAny>,
        ) -> PyResult<()> {
            let i: usize = i.downcast::<PyInt>()?.extract()?;
            let j: usize = j.downcast::<PyInt>()?.extract()?;
            let mut fig = self_.fig.lock().unwrap();
            fig.plot_rows[i].plots[j].add_line(x, y)
        }
    }
}
