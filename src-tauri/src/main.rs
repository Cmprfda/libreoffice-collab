// Hide the console window in release builds. Without this, double-clicking the
// app on Windows would flash a black cmd window behind it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    libreoffice_collab_lib::run();
}
