use clap::Command;

fn main() {
    let _matchs = Command::new("echoes")
        .version("0.1.0")
        .author("BioErrorLog <bioerrorlog.contact@gmail.com>")
        .about("Rust echo")
        .get_matches();
}
