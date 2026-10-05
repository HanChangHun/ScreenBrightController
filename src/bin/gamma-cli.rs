fn main() {
    if let Err(e) = gamma_dimmer::cli::run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
