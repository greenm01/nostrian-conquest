//! Python bindings for [`nc_ai::BatchEnv`]: a batch of seeded games whose
//! observations, masks and results come back as numpy arrays that own the
//! Rust buffers, without a copy. Choices go in as int64 arrays shaped as
//! the masks describe. Games run with the GIL released.

use nc_ai::{BatchActions, BatchEnv, Table, Tensor};
use numpy::{Element, PyArray1, PyArrayMethods, PyReadonlyArrayDyn};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Seeded games stepped together; every seat is chosen by the caller.
#[pyclass(module = "nc_ai_env")]
struct Env {
    players: u8,
    years: u16,
    threads: usize,
    batch: Option<BatchEnv>,
}

#[pymethods]
impl Env {
    /// `players` seats per game, each game ending after `years` years or a
    /// declared winner. `threads` 0 uses every core.
    #[new]
    #[pyo3(signature = (players, years, threads = 0))]
    fn new(players: u8, years: u16, threads: usize) -> PyResult<Self> {
        if !(2..=25).contains(&players) {
            return Err(PyValueError::new_err("players must be 2 to 25"));
        }
        Ok(Self {
            players,
            years,
            threads,
            batch: None,
        })
    }

    /// Start one fresh game per map seed. Campaign seeds default to the map
    /// seeds.
    #[pyo3(signature = (map_seeds, campaign_seeds = None))]
    fn reset(
        &mut self,
        py: Python<'_>,
        map_seeds: Vec<u64>,
        campaign_seeds: Option<Vec<u64>>,
    ) -> PyResult<()> {
        let campaign_seeds = campaign_seeds.unwrap_or_else(|| map_seeds.clone());
        if campaign_seeds.len() != map_seeds.len() {
            return Err(PyValueError::new_err(
                "campaign_seeds must match map_seeds in length",
            ));
        }
        let (players, years, threads) = (self.players, self.years, self.threads);
        self.batch =
            Some(py.detach(|| BatchEnv::new(players, &map_seeds, &campaign_seeds, years, threads)));
        Ok(())
    }

    /// Every seat's observation: `global`, then `planets`, `fleets`,
    /// `contacts` and `starbases` with a `*_valid` row flag each.
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let obs = py.detach(|| batch.observe());
        let out = PyDict::new(py);
        out.set_item("global", array(py, obs.global)?)?;
        for (name, table) in [
            ("planets", obs.planets),
            ("fleets", obs.fleets),
            ("contacts", obs.contacts),
            ("starbases", obs.starbases),
        ] {
            put_table(&out, name, table)?;
        }
        Ok(out)
    }

    /// Every seat's legal choices as bool arrays: `diplomacy`, `build`,
    /// `mission`, `target`, `roe` and `load`.
    fn masks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let masks = py.detach(|| batch.masks());
        let out = PyDict::new(py);
        for (name, tensor) in [
            ("diplomacy", masks.diplomacy),
            ("build", masks.build),
            ("mission", masks.mission),
            ("target", masks.target),
            ("roe", masks.roe),
            ("load", masks.load),
        ] {
            out.set_item(name, array(py, tensor)?)?;
        }
        Ok(out)
    }

    /// Apply a dict of int64 choice arrays (`tax`, `diplomacy`, `build`,
    /// `amount`, `mission`, `target`, `speed`, `roe`, `load`) and play a
    /// year. Returns `reward`, `done`, `production` and `worlds`. A choice
    /// outside the masks raises ValueError and changes nothing.
    fn step<'py>(
        &mut self,
        py: Python<'py>,
        actions: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let get = |name: &str| -> PyResult<PyReadonlyArrayDyn<'py, i64>> {
            actions
                .get_item(name)?
                .ok_or_else(|| PyKeyError::new_err(name.to_owned()))?
                .extract()
                .map_err(|err| PyValueError::new_err(format!("{name}: {err}")))
        };
        let arrays = [
            get("tax")?,
            get("diplomacy")?,
            get("build")?,
            get("amount")?,
            get("mission")?,
            get("target")?,
            get("speed")?,
            get("roe")?,
            get("load")?,
        ];
        let slices = arrays
            .iter()
            .map(|array| array.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PyValueError::new_err("choice arrays must be contiguous"))?;
        let actions = BatchActions {
            tax: slices[0],
            diplomacy: slices[1],
            build: slices[2],
            amount: slices[3],
            mission: slices[4],
            target: slices[5],
            speed: slices[6],
            roe: slices[7],
            load: slices[8],
        };
        let batch = self
            .batch
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("call reset first"))?;
        let result = py
            .detach(|| batch.step(&actions))
            .map_err(PyValueError::new_err)?;
        let out = PyDict::new(py);
        out.set_item("reward", array(py, result.reward)?)?;
        out.set_item("done", array(py, result.done)?)?;
        out.set_item("production", array(py, result.production)?)?;
        out.set_item("worlds", array(py, result.worlds)?)?;
        Ok(out)
    }

    #[getter]
    fn players(&self) -> u8 {
        self.players
    }

    #[getter]
    fn games(&self) -> PyResult<usize> {
        Ok(self.batch()?.games())
    }

    /// The padded world count, `W`.
    #[getter]
    fn worlds(&self) -> PyResult<usize> {
        Ok(self.batch()?.worlds())
    }

    /// The padded fleet count, `F`, for the coming year.
    #[getter]
    fn fleets(&self) -> PyResult<usize> {
        Ok(self.batch()?.fleets())
    }

    /// Seconds spent building seat masks since reset, and how many seat
    /// masks were built.
    #[getter]
    fn mask_cost(&self) -> PyResult<(f64, u64)> {
        let (time, count) = self.batch()?.mask_cost();
        Ok((time.as_secs_f64(), count))
    }
}

impl Env {
    fn batch(&self) -> PyResult<&BatchEnv> {
        self.batch
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("call reset first"))
    }
}

/// Hand `tensor`'s buffer to numpy and give it its shape.
fn array<'py, T: Element>(py: Python<'py>, tensor: Tensor<T>) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyArray1::from_vec(py, tensor.data)
        .reshape(tensor.shape)?
        .into_any())
}

fn put_table(out: &Bound<'_, PyDict>, name: &str, table: Table) -> PyResult<()> {
    let py = out.py();
    out.set_item(name, array(py, table.rows)?)?;
    out.set_item(format!("{name}_valid"), array(py, table.valid)?)
}

#[pymodule]
fn nc_ai_env(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Env>()?;
    m.add("TAX_CHOICES", nc_ai::TAX_CHOICES)?;
    m.add("TAX_STEP", nc_ai::TAX_STEP)?;
    m.add("DIPLOMACY_CHOICES", nc_ai::DIPLOMACY_CHOICES)?;
    m.add("BUILD_CHOICES", nc_ai::BUILD_CHOICES)?;
    m.add("BUILD_AMOUNT_CHOICES", nc_ai::BUILD_AMOUNT_CHOICES)?;
    m.add("MISSION_CHOICES", nc_ai::MISSION_CHOICES)?;
    m.add("SPEED_CHOICES", nc_ai::SPEED_CHOICES)?;
    m.add("ROE_CHOICES", nc_ai::ROE_CHOICES)?;
    m.add("GLOBAL_WIDTH", nc_ai::GLOBAL_WIDTH)?;
    m.add("PLANET_WIDTH", nc_ai::PLANET_WIDTH)?;
    m.add("FLEET_WIDTH", nc_ai::FLEET_WIDTH)?;
    m.add("CONTACT_WIDTH", nc_ai::CONTACT_WIDTH)?;
    m.add("STARBASE_WIDTH", nc_ai::STARBASE_WIDTH)?;
    Ok(())
}
