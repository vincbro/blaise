use std::{collections::BinaryHeap, rc::Rc};

use crate::engine::{
    AVERAGE_STOP_DISTANCE, Engine, Stop,
    geo::{Coordinate, Distance},
};

#[derive(Debug, Clone)]
pub enum NodeType {
    Stop(Stop),
    Coordinate(Coordinate),
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Coordinate(Default::default())
    }
}

impl NodeType {
    pub fn distance(&self, other: &Self) -> Distance {
        self.coordinate().distance(other.coordinate())
    }

    pub fn coordinate(&self) -> &Coordinate {
        match self {
            NodeType::Stop(stop) => &stop.coordinate,
            NodeType::Coordinate(coordinate) => coordinate,
        }
    }
}

type RouterNode = Rc<Node>;

#[derive(Default, Debug, Clone)]
pub struct Node {
    node_type: NodeType,
    g_score: Distance,
    h_score: Distance,
    parent: Option<RouterNode>,
}

impl Node {
    pub fn distance(&self, other: &Self) -> Distance {
        self.coordinate().distance(other.coordinate())
    }

    pub fn coordinate(&self) -> &Coordinate {
        self.node_type.coordinate()
    }

    pub fn cost(&self) -> i64 {
        (self.h_score + self.g_score).as_meters().floor() as i64
    }
}
impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cost() == other.cost()
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost().cmp(&self.cost())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Router {
    engine: Engine,
    heap: BinaryHeap<RouterNode>,
    start: RouterNode,
    start_type: NodeType,
    end: RouterNode,
    end_type: NodeType,
    neigbour_distance: Distance,
}

impl Router {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            heap: Default::default(),
            neigbour_distance: AVERAGE_STOP_DISTANCE,
            start: Default::default(),
            end: Default::default(),
            start_type: Default::default(),
            end_type: Default::default(),
        }
    }

    pub fn with_start_coordinate(mut self, coordinate: Coordinate) -> Self {
        self.start_type = NodeType::Coordinate(coordinate);
        self
    }

    pub fn with_start_stop(mut self, stop: Stop) -> Self {
        self.start_type = NodeType::Stop(stop);
        self
    }

    pub fn with_end_coordinate(mut self, coordinate: Coordinate) -> Self {
        self.end_type = NodeType::Coordinate(coordinate);
        self
    }

    pub fn with_end_stop(mut self, stop: Stop) -> Self {
        self.end_type = NodeType::Stop(stop);
        self
    }

    pub fn with_neigbour_distance(mut self, distance: Distance) -> Self {
        self.neigbour_distance = distance;
        self
    }

    pub fn run(&mut self) {
        let start_to_end_dist = self.start_type.distance(&self.end_type);
        self.end = Node {
            node_type: self.end_type.clone(),
            g_score: Default::default(),
            h_score: Default::default(),
            parent: None,
        }
        .into();
        self.start = Node {
            node_type: self.start_type.clone(),
            g_score: Default::default(),
            h_score: start_to_end_dist,
            parent: None,
        }
        .into();

        self.add_neigbours(self.start.clone());
    }

    fn add_neigbours(&mut self, node: RouterNode) {
        // Distance neighbours
        self.engine
            .stops_by_coordinate(node.coordinate(), self.neigbour_distance)
            .into_iter()
            .filter(|stop| self.engine.trips_by_stop_id(&stop.id).is_some())
            .for_each(|stop| {
                let dist_to_goal = stop.coordinate.distance(self.end.coordinate());
                let dist_to_parent = node.g_score + stop.coordinate.distance(node.coordinate());
                let node: RouterNode = Node {
                    node_type: NodeType::Stop(stop.clone()),
                    g_score: dist_to_parent,
                    h_score: dist_to_goal,
                    parent: Some(node.clone()),
                }
                .into();
                self.heap.push(node);
            });
    }
}
