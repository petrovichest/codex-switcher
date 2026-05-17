//! Codex Switcher - Multi-account manager for Codex CLI

pub mod api;
pub mod auth;
pub mod commands;
pub mod settings;
pub mod types;
pub mod web;

use commands::{
    add_account_from_file, cancel_login, check_codex_processes, clear_proxy_settings,
    complete_device_login, complete_login, delete_account, export_accounts_full_encrypted_file,
    export_accounts_slim_text, get_active_account_info, get_masked_account_ids, get_proxy_settings,
    get_usage, import_accounts_full_encrypted_file, import_accounts_slim_text, list_accounts,
    refresh_account_metadata, refresh_all_accounts_usage, rename_account, set_masked_account_ids,
    set_proxy_settings, start_device_login, start_login, switch_account, test_proxy_settings,
    warmup_account, warmup_all_accounts,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(err) = settings::apply_stored_proxy_environment() {
        eprintln!("[Settings] Failed to apply stored proxy settings: {err}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Account management
            list_accounts,
            get_active_account_info,
            add_account_from_file,
            switch_account,
            delete_account,
            rename_account,
            export_accounts_slim_text,
            import_accounts_slim_text,
            export_accounts_full_encrypted_file,
            import_accounts_full_encrypted_file,
            // Masked accounts
            get_masked_account_ids,
            set_masked_account_ids,
            // Settings
            get_proxy_settings,
            set_proxy_settings,
            clear_proxy_settings,
            test_proxy_settings,
            // OAuth
            start_login,
            complete_login,
            start_device_login,
            complete_device_login,
            cancel_login,
            // Usage
            get_usage,
            refresh_account_metadata,
            refresh_all_accounts_usage,
            warmup_account,
            warmup_all_accounts,
            // Process detection
            check_codex_processes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
