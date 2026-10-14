//! Row-major arrays, padding and the thread pool the batch steps on.

use std::thread;

/// A row-major array and its shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor<T> {
    pub data: Vec<T>,
    pub shape: Vec<usize>,
}

/// One padded entity table and its row flags.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// `[games, seats, rows, width]`.
    pub rows: Tensor<f32>,
    /// `[games, seats, rows]`.
    pub valid: Tensor<bool>,
}

pub(crate) fn falses(shape: &[usize]) -> Tensor<bool> {
    Tensor {
        data: vec![false; shape.iter().product()],
        shape: shape.to_vec(),
    }
}

/// Copy `row` into row `idx` of a tensor whose last axis is `row.len()` or
/// longer.
pub(crate) fn put(tensor: &mut Tensor<bool>, idx: usize, row: &[bool]) {
    let width = *tensor.shape.last().expect("a tensor has an axis");
    tensor.data[idx * width..idx * width + row.len()].copy_from_slice(row);
}

/// Pad each view's rows of `width` to `rows` rows.
pub(crate) fn pad_table<'a>(
    lead: [usize; 2],
    views: impl Iterator<Item = &'a [f32]>,
    width: usize,
    rows: usize,
) -> Table {
    let mut table = Table {
        rows: Tensor {
            data: Vec::with_capacity(lead[0] * lead[1] * rows * width),
            shape: vec![lead[0], lead[1], rows, width],
        },
        valid: Tensor {
            data: Vec::with_capacity(lead[0] * lead[1] * rows),
            shape: vec![lead[0], lead[1], rows],
        },
    };
    for view in views {
        let count = view.len() / width;
        table.rows.data.extend_from_slice(view);
        table
            .rows
            .data
            .resize(table.rows.data.len() + (rows - count) * width, 0.0);
        table.valid.data.extend((0..rows).map(|row| row < count));
    }
    table
}

/// Run `f` on every item, the items split into contiguous runs over
/// `threads` threads.
pub(crate) fn par_for_each_mut<T: Send>(
    items: &mut [T],
    threads: usize,
    f: impl Fn(usize, &mut T) + Sync,
) {
    let chunk = items.len().div_ceil(threads.max(1)).max(1);
    thread::scope(|scope| {
        for (run, part) in items.chunks_mut(chunk).enumerate() {
            let f = &f;
            scope.spawn(move || {
                for (offset, item) in part.iter_mut().enumerate() {
                    f(run * chunk + offset, item);
                }
            });
        }
    });
}
