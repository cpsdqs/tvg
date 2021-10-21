fn main() {
    let mut args = std::env::args();
    args.next().expect("no exec arg");
    let file_path = args.next().expect("missing file path argument");

    let file = std::fs::File::open(file_path).expect("failed to read file");
    let tvg = match tvg::read::read(file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(-1);
        }
    };

    for item in tvg {
        println!("{:02x?}", item);
    }
}
