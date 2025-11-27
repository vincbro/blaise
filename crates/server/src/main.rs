fn main() {
    // let config = ontrack::gtfs::Config::default();
    // let loader = ontrack::gtfs::GtfsLoader::new(config).load_from_zip("");
    println!("Hello, world!");
    let res = ontrack::add(1, 2);
    println!("Ontrack: {res}");
}
