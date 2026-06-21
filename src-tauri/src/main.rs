// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 导入日志宏（从库 crate 中）
use learn_tauri_lib::log_info;

fn main() {
    log_info!("Application started from main.rs");
    learn_tauri_lib::run()
}