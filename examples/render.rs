use std::io::Write;

use rbc::game::{BoardState};

fn main() {
    println!("hi");
    let mut fout = std::fs::File::create("logs/render_example.html").unwrap();
    writeln!(fout, "{}", rbc::html::PREAMBLE).unwrap();
    let s = BoardState::initial();
    writeln!(fout, "{}", s.to_html()).unwrap();
    writeln!(fout, "<table>").unwrap();
    writeln!(fout, "<tr>").unwrap();
    for _ in 0..5 {
        writeln!(fout, "<td>{}</td>", s.to_html()).unwrap();
    }
    writeln!(fout, "</tr>").unwrap();
    writeln!(fout, "</table>").unwrap();
    writeln!(fout, "Hello, world!").unwrap();
}
