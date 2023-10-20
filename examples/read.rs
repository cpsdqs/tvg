use tvg::read::FileData;

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
        match item {
            FileData::Main(items) => {
                println!("<main>");
                for item in items {
                    println!("{item:02x?}");
                }
                println!("</main>");
            }
            item => println!("{item:02x?}"),
        }
    }
}
