// No console window in release builds (launched from Steam / TV).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    battle_tanks::run();
}
