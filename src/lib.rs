use egui::{Context, ScrollArea};
use egui_inspect::EguiInspect;
use egui_plot::{Line, PlotPoints};
use pyo3::{
    exceptions::PyIndexError,
    prelude::*,
    types::{PyInt, PyList},
};
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

struct PlotsWindow {
    figs: Arc<Mutex<HashMap<String, Figure>>>,
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

#[pymodule]
mod eguiplotlib {

    use pyo3::types::PyString;

    use self::run_native::run_native;

    use super::*;

    #[pyclass]
    struct FigureCanvas {
        join_handle: JoinHandle<()>,
        figs: Arc<Mutex<HashMap<String, Figure>>>,
    }

    #[pyclass]
    struct FigHandle {
        // TODO: These will keep figure data alive even when canvas is dropped, make weakrefs?
        figs: Arc<Mutex<HashMap<String, Figure>>>,
        figkey: String,
    }

    #[pyclass]
    struct PlotHandle {
        figs: Arc<Mutex<HashMap<String, Figure>>>,
        figkey: String,
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

        fn is_running(self_: PyRef<'_, Self>) -> bool {
            !self_.join_handle.is_finished()
        }

        fn add_figure(
            self_: PyRef<'_, Self>,
            s: Bound<'_, PyString>,
            n: Bound<'_, PyInt>,
            m: Bound<'_, PyInt>,
        ) -> PyResult<FigHandle> {
            let s: String = s.extract()?;
            let n: usize = n.extract()?;
            let m: usize = m.extract()?;
            {
                let mut figs = self_.figs.lock().unwrap();
                let fig = Figure::new(n, m);
                figs.insert(s.clone(), fig);
            }
            Ok(FigHandle {
                figkey: s,
                figs: self_.figs.clone(),
            })
        }
    }

    #[pymethods]
    impl FigHandle {
        /// Aquire a handle to the plot at grid coords (i, j).
        fn plot(
            self_: PyRef<'_, Self>,
            i: Bound<'_, PyInt>,
            j: Bound<'_, PyInt>,
        ) -> PyResult<PlotHandle> {
            let i: usize = i.extract()?;
            let j: usize = j.extract()?;
            match self_.figs.lock() {
                Ok(mut figs) => {
                    let fig = figs.get_mut(&self_.figkey).unwrap();
                    let n = fig.plot_rows.len();
                    let pr = fig
                        .plot_rows
                        .get_mut(i)
                        .ok_or(PyErr::new::<PyIndexError, _>(format!(
                            "Index {i} invalid for rows {n}.",
                        )))?;
                    let m = pr.plots.len();
                    let _ = pr
                        .plots
                        .get_mut(j)
                        .ok_or(PyErr::new::<PyIndexError, _>(format!(
                            "Index {j} invalid for {m} plots in row.",
                        )))?;
                    Ok(PlotHandle {
                        figs: self_.figs.clone(),
                        figkey: self_.figkey.clone(),
                        row: i,
                        col: j,
                    })
                }
                Err(_) => Err(PyErr::new::<PyIndexError, _>(format!(
                    "Handle no longer valid.",
                ))),
            }
        }
    }

    #[pymethods]
    impl PlotHandle {
        fn add_line(
            self_: PyRef<'_, Self>,
            x: Bound<'_, PyList>,
            y: Bound<'_, PyList>,
        ) -> PyResult<()> {
            let xy_data = x
                .iter()
                .zip(y.iter())
                .filter_map(|(xf, yf)| match (xf.extract(), yf.extract()) {
                    (Ok(x), Ok(y)) => Some([x, y]),
                    _ => None,
                })
                .collect();
            match self_.figs.lock() {
                Ok(mut figs) => match figs.get_mut(&self_.figkey) {
                    Some(fig) => {
                        let p = &mut fig.plot_rows[self_.row].plots[self_.col];
                        p.add_line(xy_data)
                    }
                    None => Err(PyErr::new::<PyIndexError, _>(format!(
                        "No figure with key \"{}\", handle is old?",
                        self_.figkey
                    ))),
                },
                Err(_) => Err(PyErr::new::<PyIndexError, _>(format!(
                    "Handle no longer valid.",
                ))),
            }
            // TODO: Signal request repaint on modification?
        }
    }
}
