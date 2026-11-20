#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    #[cfg(target_os = "windows")]
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--self-test-voice")
    {
        std::process::exit(companion_app_lib::voice_self_test());
    }
    companion_app_lib::run()
}
