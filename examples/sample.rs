//! This is a silly little text rendering program for the purpose of illustrating what flame graphs
//! look like.

use no_nonsense_flamegraphs::outln;
use std::thread::sleep;
use std::time::Duration;

const LINE_WIDTH: usize = 40;

fn main() {
    outln!("main");

    let title = "Sample Flame Graph".to_owned();
    let paragraph = "This is a silly little text rendering program for the purpose of illustrating what flame graphs look like.".to_owned();
    render_title(title);
    render_paragraph(paragraph);
}

fn render_title(title: String) {
    outln!("render_title");
    sleep(Duration::from_millis(20));

    for ch in title.chars() {
        for ch in ch.to_uppercase() {
            render_char(ch);
        }
    }
    render_char('\n');
    for _ in 0..title.len() {
        render_char('=');
    }
    render_char('\n');
    render_char('\n');
}

fn render_paragraph(paragraph: String) {
    outln!("render_paragraph");
    sleep(Duration::from_millis(15));

    let lines = split_lines(paragraph);
    for line in lines {
        for ch in line.chars() {
            render_char(ch);
        }
        render_char('\n');
    }
}

fn split_lines(paragraph: String) -> Vec<String> {
    outln!("split_lines");
    sleep(Duration::from_millis(50));

    let words = paragraph.split(' ');
    let mut lines = vec![String::new()];
    for word in words {
        if lines.last().unwrap().len() + 1 + word.len() > LINE_WIDTH {
            let last_line = lines.last_mut().unwrap();
            if last_line.ends_with(' ') {
                last_line.pop();
            }
            lines.push(String::new());
        }
        let last_line = lines.last_mut().unwrap();
        last_line.push_str(word);
        last_line.push(' ');
    }
    lines
}

fn render_char(ch: char) {
    outln!("render_char");
    sleep(Duration::from_millis(1));

    print!("{}", ch);
}
