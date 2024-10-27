// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

#[cfg(target_os = "windows")]
fn main() {
    let args: Vec<String> = env::args().collect();

    glassui_lib::run(args.contains(&"noSettings".to_string()))
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("This application is only supported on Windows.");
}