//! Usage query Tauri commands

use crate::api::usage::{
    fetch_chatgpt_account_metadata, get_account_usage, refresh_all_usage,
    warmup_account as send_warmup,
};
use crate::auth::{
    ensure_chatgpt_tokens_fresh, get_account, load_accounts, refresh_chatgpt_tokens,
    update_account_metadata,
};
use crate::types::{AccountInfo, AuthData, UsageInfo, WarmupSummary};
use anyhow::Error as AnyhowError;
use futures::{stream, StreamExt};

/// Get usage info for a specific account
#[tauri::command]
pub async fn get_usage(account_id: String) -> Result<UsageInfo, String> {
    let account = get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {account_id}"))?;

    get_account_usage(&account).await.map_err(|e| e.to_string())
}

/// Refresh account metadata for a specific account.
/// For ChatGPT accounts this only refreshes OAuth tokens when they are expired
/// or when the metadata endpoint rejects the current access token.
/// For API key accounts this is a no-op.
#[tauri::command]
pub async fn refresh_account_metadata(account_id: String) -> Result<AccountInfo, String> {
    let account = get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {account_id}"))?;

    let updated = match &account.auth_data {
        AuthData::ApiKey { .. } => account,
        AuthData::ChatGPT { .. } => {
            let fresh_account = ensure_chatgpt_tokens_fresh(&account)
                .await
                .map_err(|e| e.to_string())?;
            let live_metadata = match fetch_chatgpt_account_metadata(&fresh_account).await {
                Ok(metadata) => metadata,
                Err(err) if is_unauthorized_error(&err) => {
                    let refreshed = refresh_chatgpt_tokens(&fresh_account)
                        .await
                        .map_err(|e| e.to_string())?;
                    fetch_chatgpt_account_metadata(&refreshed)
                        .await
                        .map_err(|e| e.to_string())?
                }
                Err(err) => return Err(err.to_string()),
            };

            update_account_metadata(
                &account_id,
                None,
                None,
                live_metadata.plan_type,
                Some(live_metadata.subscription_expires_at),
            )
            .map_err(|e| e.to_string())?
        }
    };

    let store = load_accounts().map_err(|e| e.to_string())?;
    let active_id = store.active_account_id.as_deref();
    Ok(AccountInfo::from_stored(&updated, active_id))
}

fn is_unauthorized_error(err: &AnyhowError) -> bool {
    err.chain()
        .any(|cause| cause.to_string().contains("401 Unauthorized"))
}

/// Refresh usage info for all accounts
#[tauri::command]
pub async fn refresh_all_accounts_usage() -> Result<Vec<UsageInfo>, String> {
    let store = load_accounts().map_err(|e| e.to_string())?;
    Ok(refresh_all_usage(&store.accounts).await)
}

/// Send a minimal warm-up request for one account
#[tauri::command]
pub async fn warmup_account(account_id: String) -> Result<(), String> {
    let account = get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {account_id}"))?;

    send_warmup(&account).await.map_err(|e| e.to_string())
}

/// Send minimal warm-up requests for all accounts
#[tauri::command]
pub async fn warmup_all_accounts() -> Result<WarmupSummary, String> {
    let store = load_accounts().map_err(|e| e.to_string())?;
    let total_accounts = store.accounts.len();
    let concurrency = total_accounts.min(10).max(1);

    let results: Vec<(String, bool)> = stream::iter(store.accounts.into_iter())
        .map(|account| async move {
            let account_id = account.id.clone();
            let failed = send_warmup(&account).await.is_err();
            (account_id, failed)
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let failed_account_ids = results
        .into_iter()
        .filter_map(|(account_id, failed)| failed.then_some(account_id))
        .collect::<Vec<_>>();

    let warmed_accounts = total_accounts.saturating_sub(failed_account_ids.len());
    Ok(WarmupSummary {
        total_accounts,
        warmed_accounts,
        failed_account_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::is_unauthorized_error;

    #[test]
    fn detects_unauthorized_metadata_error() {
        let err = anyhow::anyhow!(
            "Accounts check API error: 401 Unauthorized - {{\"error\":\"expired\"}}"
        );

        assert!(is_unauthorized_error(&err));
    }

    #[test]
    fn ignores_non_unauthorized_metadata_error() {
        let err = anyhow::anyhow!("Accounts check API error: 500 Internal Server Error");

        assert!(!is_unauthorized_error(&err));
    }
}
