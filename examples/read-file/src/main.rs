use std::fs::read;
use std::io;

fn main() -> io::Result<()> {
    let data = String::from_utf8(read("data.txt")?).unwrap();
    println!("File contents: {}", data);
    Ok(())
}
