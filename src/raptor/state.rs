use crate::{raptor::location::Point, shared::Time};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ParentType {
    Transit(u32),
    Transfer,
    Walk,
}

impl ParentType {
    pub fn is_transit(&self) -> bool {
        matches!(self, ParentType::Transit(_))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Parent {
    pub from: Point,
    pub to: Point,
    pub parent_type: ParentType,
    pub departure_time: Time,
    pub arrival_time: Time,
}

impl Parent {
    pub fn new_transit(
        from: Point,
        to: Point,
        trip: u32,
        departure_time: Time,
        arrival_time: Time,
    ) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Transit(trip),
            departure_time,
            arrival_time,
        }
    }
    pub fn new_transfer(from: Point, to: Point, departure_time: Time, arrival_time: Time) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Transfer,
            departure_time,
            arrival_time,
        }
    }
    pub fn new_walk(from: Point, to: Point, departure_time: Time, arrival_time: Time) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Walk,
            departure_time,
            arrival_time,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Update {
    pub stop_idx: u32,
    pub arrival_time: Time,
    pub parent: Parent,
}

impl Update {
    pub fn new(stop_idx: u32, arrival_time: Time, parent: Parent) -> Self {
        Self {
            stop_idx,
            arrival_time,
            parent,
        }
    }
}

pub struct LazyBuffer<T> {
    buffer: Option<Vec<T>>,
    capacity: usize,
}

impl<T> LazyBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: None,
            capacity,
        }
    }

    pub fn push(&mut self, value: T) {
        if let Some(buffer) = &mut self.buffer {
            buffer.push(value);
        } else {
            let mut buffer = Vec::with_capacity(self.capacity);
            buffer.push(value);
            self.buffer = Some(buffer);
        }
    }

    pub fn get(mut self) -> Option<Vec<T>> {
        self.buffer.take()
    }
}
