use std::process::Command;

pub mod configs;

use configs::config::SHELL;

fn main() {
    let output = Command::new(SHELL).arg("-c").arg("dir").output().unwrap();

    let mut s = String::from("hello");

    // let r1 = &s; // обычная ссылка
    let r2 = &mut s; // ещё одна ссылка
    println!("{r2}"); // r1 и r2 используются здесь

    // <- после этой строки r1 и r2 больше нигде не нужны
    // компилятор "видит", что они умирают прямо тут
    println!("{r2}");
    // let r3 = &mut s; // можно взять мутабельную ссылку
    r2.push_str(" string");
    
}

fn takes_ownership(s1: &mut String) {
    s1.push_str("world")
}

fn makes_copy(some_integer: i32) {
    println!("{some_integer}");
}