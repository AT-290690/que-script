use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io;
use std::io::Write;

fn compress(data: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data.as_bytes())
        .expect("Failed to compress data");
    encoder.finish().expect("Failed to finish compression")
}

fn dump_wrapped_libs(expr: &str, path: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(
        file,
        "use std::io::Read;fn decompress(compressed: &[u8]) -> crate::parser::Expression {{use flate2::read::GzDecoder;let mut decoder = GzDecoder::new(compressed);let mut decompressed_data = String::new();decoder.read_to_string(&mut decompressed_data).expect(\"Failed to decompress data\");let expressions =crate::parser::parse(&decompressed_data).expect(\"Failed to parse decompressed data\");expressions.first().expect(\"No expressions returned\").clone()}}"
    )?;
    let compressed_code = compress(expr);
    writeln!(file, "pub fn load_ast() -> crate::parser::Expression {{")?;
    writeln!(file, "decompress(&{:?})", compressed_code)?;
    writeln!(file, "}}")?;
    Ok(())
}

fn run() -> io::Result<()> {
    let combined = format!(
        "{}\n{}\n{}\n{}",
        fs::read_to_string("./lisp/const.lisp")?,
        fs::read_to_string("./lisp/std.lisp")?,
        fs::read_to_string("./lisp/fp.lisp")?,
        fs::read_to_string("./lisp/ds.lisp")?
    );
    let built = que::parser::build(&combined)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    dump_wrapped_libs(&built.to_lisp(), "./src/baked.rs")
}

fn main() {
    if let Err(err) = run() {
        eprintln!("quebake error: {}", err);
        std::process::exit(1);
    }
}
