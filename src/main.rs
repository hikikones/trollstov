mod audio;
mod terminal;

fn main() {
    let dir = std::env::args().last().expect("expected dir path");
    let db = audio::Database::new(dir);
    println!("{db:?}");
}
