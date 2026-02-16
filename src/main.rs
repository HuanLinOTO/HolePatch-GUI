#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod core;
mod gui;
mod profile;

fn main() {
    gui::run_app();
}
