pub struct Config {
    pub stops_path: String,
    pub areas_path: String,
    pub routes_path: String,
    pub agency_path: String,
    pub stop_areas_path: String,
    pub transfers_path: String,
    pub stop_times_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stops_path: "stops.txt".into(),
            areas_path: "areas.txt".into(),
            routes_path: "routes.txt".into(),
            agency_path: "agency.txt".into(),
            stop_areas_path: "stop_areas.txt".into(),
            transfers_path: "transfers.txt".into(),
            stop_times_path: "stop_times.txt".into(),
        }
    }
}
