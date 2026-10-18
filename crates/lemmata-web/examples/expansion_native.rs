fn main() {
    let capture = std::fs::read(std::env::args().nth(1).expect("capture")).expect("read");
    print!("{}", lemmata_web::expansion_hashes(&capture).expect("hash"));
}
