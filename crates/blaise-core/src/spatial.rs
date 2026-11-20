use gtfs_bin_core::{
    consumer::Consumer,
    models::{Coordinate, StopIndex},
};

/// Meters in a degree of latitude.
const DEGREE_OF_LATITUDE: f64 = 111_320.0;
/// The side of a grid cell in meters.
const CELL_SIZE: f64 = 1000.0;

/// The most stops a grid can hold for `check_invariants` to read them all.
const CHECK_LIMIT: usize = 200_000;

/// A stop with its position, kept in the grid so a search needn't look it up.
#[derive(Debug, Clone, Copy)]
struct GridStop {
    stop: StopIndex,
    latitude: f64,
    longitude: f64,
}

/// The stops with a position, in a grid of square cells, to find the ones near a coordinate.
///
/// The stops are stored cell by cell. `offsets[cell]..offsets[cell + 1]` is the run of `stops` in
/// a cell, and cells are numbered row by row, so a run of cells in one row is also a single run
/// of `stops`. After every public call:
/// - There is an offset for the start of every cell, and one for the end of the last.
/// - The offsets never decrease, from 0 to the number of stops.
/// - Each stop is in the run of the cell its position falls in.
#[derive(Debug)]
pub struct SpatialGrid {
    offsets: Box<[u32]>,
    stops: Box<[GridStop]>,
    min_longitude: f64,
    min_latitude: f64,
    columns: usize,
    rows: usize,
    /// How much shorter a degree of longitude is than a degree of latitude, in the middle of the
    /// stops.
    longitude_scale: f64,
}

impl SpatialGrid {
    #[must_use]
    pub fn new(consumer: &Consumer<'_>) -> Self {
        let positioned: Vec<GridStop> = consumer
            .stops()
            .iter()
            .filter_map(|stop| {
                let (latitude, longitude) = stop.coordinate.as_option()?.to_f64_pair();
                Some(GridStop {
                    stop: stop.index,
                    latitude,
                    longitude,
                })
            })
            .collect();

        let bounds = positioned.iter().fold(None, |bounds, stop| {
            let (min_latitude, max_latitude, min_longitude, max_longitude) =
                bounds.unwrap_or((stop.latitude, stop.latitude, stop.longitude, stop.longitude));
            Some((
                min_latitude.min(stop.latitude),
                max_latitude.max(stop.latitude),
                min_longitude.min(stop.longitude),
                max_longitude.max(stop.longitude),
            ))
        });
        let (min_latitude, max_latitude, min_longitude, max_longitude) = bounds.unwrap_or_default();

        let longitude_scale = f64::midpoint(min_latitude, max_latitude).to_radians().cos();
        let width = (max_longitude - min_longitude) * DEGREE_OF_LATITUDE * longitude_scale;
        let height = (max_latitude - min_latitude) * DEGREE_OF_LATITUDE;
        let columns = cell_count(width);
        let rows = cell_count(height);

        let mut grid = Self {
            offsets: Box::default(),
            stops: Box::default(),
            min_longitude,
            min_latitude,
            columns,
            rows,
            longitude_scale,
        };
        grid.fill(positioned);
        grid.check_invariants();
        grid
    }

    /// The stops within `radius` meters of a coordinate.
    pub fn iter_stops_in_radius(
        &self,
        coordinate: Coordinate,
        radius: f64,
    ) -> impl Iterator<Item = StopIndex> + '_ {
        let (x, y) = self.meters(coordinate.latitude_f64(), coordinate.longitude_f64());
        let first_column = cell_of(x - radius, self.columns).saturating_sub(1);
        let last_column = (cell_of(x + radius, self.columns) + 1).min(self.columns - 1);
        let first_row = cell_of(y - radius, self.rows).saturating_sub(1);
        let last_row = (cell_of(y + radius, self.rows) + 1).min(self.rows - 1);

        // The cells of one row are next to each other, so they are one run of stops.
        (first_row..=last_row)
            .flat_map(move |row| {
                let first = self.offsets[row * self.columns + first_column];
                let last = self.offsets[row * self.columns + last_column + 1];
                &self.stops[first as usize..last as usize]
            })
            .filter(move |stop| {
                let (stop_x, stop_y) = self.meters(stop.latitude, stop.longitude);
                let (east, north) = (x - stop_x, y - stop_y);
                east * east + north * north <= radius * radius
            })
            .map(|stop| stop.stop)
    }

    /// Sorts the stops into cells and records where each cell's run starts.
    fn fill(&mut self, mut positioned: Vec<GridStop>) {
        positioned.sort_by_cached_key(|stop| self.cell(stop));

        let mut offsets = vec![0_u32; self.columns * self.rows + 1];
        for stop in &positioned {
            offsets[self.cell(stop) + 1] += 1;
        }
        for cell in 1..offsets.len() {
            offsets[cell] += offsets[cell - 1];
        }
        self.offsets = offsets.into_boxed_slice();
        self.stops = positioned.into_boxed_slice();
    }

    /// Panics in debug builds if a cell's run holds a stop that belongs in another cell. A grid with
    /// many stops is skipped, because the check reads every one.
    fn check_invariants(&self) {
        if !cfg!(debug_assertions) || self.stops.len() > CHECK_LIMIT {
            return;
        }
        assert_eq!(
            self.offsets.len(),
            self.columns * self.rows + 1,
            "there is an offset for the start of every cell and one for the end of the last"
        );
        assert_eq!(
            self.offsets[0], 0,
            "the first cell's run starts at the first stop"
        );
        assert_eq!(
            self.offsets.last().map(|&end| end as usize),
            Some(self.stops.len()),
            "the last cell's run ends at the last stop"
        );
        for cell in 0..self.columns * self.rows {
            let run = self.offsets[cell] as usize..self.offsets[cell + 1] as usize;
            assert!(
                run.start <= run.end,
                "a cell's run does not end before it starts"
            );
            for stop in &self.stops[run] {
                assert_eq!(
                    self.cell(stop),
                    cell,
                    "a stop is in the run of the cell it lies in"
                );
            }
        }
    }

    /// The number of the cell a stop is in.
    fn cell(&self, stop: &GridStop) -> usize {
        let (x, y) = self.meters(stop.latitude, stop.longitude);
        cell_of(y, self.rows) * self.columns + cell_of(x, self.columns)
    }

    /// A position in meters east and north of the grid's corner.
    fn meters(&self, latitude: f64, longitude: f64) -> (f64, f64) {
        (
            (longitude - self.min_longitude) * DEGREE_OF_LATITUDE * self.longitude_scale,
            (latitude - self.min_latitude) * DEGREE_OF_LATITUDE,
        )
    }
}

/// How many cells cover a length in meters, and at least one so an empty grid still has a cell.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a length is never negative, and no country has 2^52 cells across"
)]
fn cell_count(meters: f64) -> usize {
    (meters / CELL_SIZE).ceil().max(0.0) as usize + 1
}

/// The cell a distance from the grid's corner falls in, clamped to the `count` cells it has.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the value is clamped to the cell count, which is far below 2^52"
)]
fn cell_of(meters: f64, count: usize) -> usize {
    (meters / CELL_SIZE).floor().clamp(0.0, (count - 1) as f64) as usize
}

#[cfg(test)]
pub(crate) mod fuzz;
