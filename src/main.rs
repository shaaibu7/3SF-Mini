use consensus::compute_hash;

mod consensus;
mod types;

fn main() {
    let data = compute_hash(&"suleiman".to_owned());
    println!("The data: {}", data);
}
