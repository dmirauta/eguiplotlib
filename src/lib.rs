use egui::{Context, ScrollArea};
use egui_inspect::EguiInspect;
use egui_plot::{Line, PlotPoints};
use pyo3::{exceptions::PyIndexError, prelude::*};
use std::{
    collections::HashMap,
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
    fn set_line(&mut self, i: usize, xy_data: LineData) -> PyResult<()> {
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

    fn add_line(&mut self, xy_data: LineData) -> PyResult<()> {
        self.set_line(self.line_data.len(), xy_data)
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

type FigureStoreRef = Arc<Mutex<HashMap<String, Figure>>>;

struct PlotsWindow {
    figs: FigureStoreRef,
}

impl run_native::App for PlotsWindow {
    fn update(&mut self, ctx: &Context) {
        if let Ok(mut figs) = self.figs.try_lock() {
            for (name, fig) in figs.iter_mut() {
                egui::Window::new(name).show(ctx, |ui| {
                    let r = ScrollArea::both().show(ui, |ui| {
                        fig.inspect_mut("", ui);
                    });
                    fig.split_height(r.inner_rect.height());
                });
            }
        }
    }
}

/// A matplotlib style plotting library through egui
#[pymodule]
mod eguiplotlib {

    use self::run_native::run_native;

    use super::*;

    /// Holds the egui window in which floating figures can be added.
    #[pyclass]
    struct FigureCanvas {
        join_handle: JoinHandle<()>,
        figs: FigureStoreRef,
    }

    #[pyclass]
    #[derive(Clone)]
    struct FigHandle {
        // TODO: These will keep figure data alive even when canvas is dropped, make weakrefs?
        store: FigureStoreRef,
        name: String,
    }

    impl FigHandle {
        fn with_mut_ref<F, R>(&self, func: F) -> PyResult<R>
        where
            F: FnOnce(&mut Figure) -> R,
        {
            match self.store.lock() {
                Ok(mut figs) => Ok(func(figs.get_mut(&self.name).unwrap())),
                Err(_) => Err(PyErr::new::<PyIndexError, _>(format!(
                    "Handle no longer valid.",
                ))),
            }
        }
    }

    #[pyclass]
    struct PlotHandle {
        fig: FigHandle,
        row: usize,
        col: usize,
    }

    #[pymethods]
    impl FigureCanvas {
        #[new]
        fn new() -> PyResult<Self> {
            let figs = Arc::new(Mutex::new(HashMap::new()));
            let _figs = figs.clone();
            let join_handle =
                spawn(move || run_native("Egui canvas", Box::new(PlotsWindow { figs: _figs })));
            Ok(Self { join_handle, figs })
        }

        /// Check if egui canvas window is still open.
        fn is_running(self_: PyRef<'_, Self>) -> bool {
            !self_.join_handle.is_finished()
        }

        /// Add a figure containing a grid of plots to the canvas.
        #[pyo3(signature = (name, nrows=1, ncols=1))]
        fn add_figure(&self, name: String, nrows: usize, ncols: usize) -> PyResult<FigHandle> {
            let mut figs = self.figs.lock().unwrap();
            let fig = Figure::new(nrows, ncols);
            figs.insert(name.clone(), fig);
            Ok(FigHandle {
                store: self.figs.clone(),
                name,
            })
        }
    }

    #[pymethods]
    impl FigHandle {
        /// Aquire a handle to the plot at grid coords (i, j).
        #[pyo3(signature = (row=0, col=0))]
        fn plot(&self, row: usize, col: usize) -> PyResult<PlotHandle> {
            self.with_mut_ref(|fig| {
                let n = fig.plot_rows.len();
                let pr = fig
                    .plot_rows
                    .get_mut(row)
                    .ok_or(PyErr::new::<PyIndexError, _>(format!(
                        "Index {row} invalid for {n} rows.",
                    )))?;
                let m = pr.plots.len();
                let _ = pr
                    .plots
                    .get_mut(col)
                    .ok_or(PyErr::new::<PyIndexError, _>(format!(
                        "Index {col} invalid for {m} plots in row.",
                    )))?;
                Ok(PlotHandle {
                    fig: self.clone(),
                    row,
                    col,
                })
            })?
        }
    }

    #[pymethods]
    impl PlotHandle {
        /// Add a line to this plot.
        fn add_line(&self, x: Vec<f64>, y: Vec<f64>) -> PyResult<()> {
            // TODO: Accept iterator or list?
            let xy_data = x
                .into_iter()
                .zip(y.into_iter())
                .map(|(xf, yf)| [xf, yf])
                .collect();
            self.fig.with_mut_ref(|fig| {
                let p = &mut fig.plot_rows[self.row].plots[self.col];
                p.add_line(xy_data)
            })?
            // TODO: Signal request repaint on modification?
        }
    }
}
