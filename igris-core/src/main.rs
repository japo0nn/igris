use std::process::Command;

pub mod configs;

use configs::config::SHELL;

fn main() {
    let output = Command::new(SHELL).arg("-c").arg("dir").output().unwrap();

    println!("{}", String::from_utf8_lossy(&output.stdout));

    let s1 = String::from("hello");

    let s2 = String::from(", world");
    let s3 = takes_ownership(s1, s2);
    println!("{s3}");
    let x = 5;

    makes_copy(x);
}

fn takes_ownership(s1: &String, s2: &String) -> String {
    s1 + s2
}

fn makes_copy(some_integer: i32) {
    println!("{some_integer}");
}
