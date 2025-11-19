use hexagondb::parse_query;

fn main() {
    let query = String::from("SET name 'Hexagon DB!'");
    println!("{:?}",parse_query::parse_query(query));
}
