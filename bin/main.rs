use std::env;
use std::fs;
use mlisp::interpreter::run_interpreter;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    assert!(args.len() > 1, "Must supply a file path.");
    let file = fs::read_to_string(&args[1]).expect("There was an error eading the file.");

    println!("Read content: {}", file);

    run_interpreter(&file);
}
