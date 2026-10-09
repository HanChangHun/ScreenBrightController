fn main() {
    if let Err(e) =
        screen_bright_controller::cli::run(&std::env::args().skip(1).collect::<Vec<_>>())
    {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
