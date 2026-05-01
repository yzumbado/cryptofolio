use crate::cli::{AuditCommands, GlobalOptions};
use crate::error::Result;
use colored::Colorize;
use sqlx::SqlitePool;

pub async fn handle_audit_command(
    command: AuditCommands,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    match command {
        AuditCommands::Sync {
            wallet,
            chain,
            limit,
        } => handle_audit_sync(wallet, chain, limit, pool, opts).await,

        AuditCommands::Coverage { wallet, chain } => {
            handle_audit_coverage(wallet, chain, pool, opts).await
        }

        AuditCommands::Errors { limit } => handle_audit_errors(limit, pool, opts).await,
    }
}

// ---------------------------------------------------------------------------
// audit sync
// ---------------------------------------------------------------------------

/// Row from sync_audit_log joined with accounts.
struct AuditRow {
    timestamp: String,
    account_name: String,
    address: String,
    chain: String,
    provider: String,
    action: String,
    records_in: Option<i64>,
    records_new: Option<i64>,
    error: Option<String>,
    duration_ms: Option<i64>,
}

async fn handle_audit_sync(
    wallet: Option<String>,
    chain: Option<String>,
    limit: i64,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
    )> = sqlx::query_as(
        "SELECT
            sal.timestamp,
            COALESCE(a.name, sal.account_id) AS account_name,
            sal.address,
            sal.chain,
            sal.provider,
            sal.action,
            sal.records_in,
            sal.records_new,
            sal.error,
            sal.duration_ms
         FROM sync_audit_log sal
         LEFT JOIN accounts a ON a.id = sal.account_id
         WHERE (? IS NULL OR a.name = ? OR sal.account_id = ?)
           AND (? IS NULL OR sal.chain = ?)
         ORDER BY sal.timestamp DESC
         LIMIT ?",
    )
    .bind(&wallet)
    .bind(&wallet)
    .bind(&wallet)
    .bind(&chain)
    .bind(&chain)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        if !opts.quiet {
            println!("No sync audit entries found.");
            println!("Run `cryptofolio wallet sync --all` to populate the audit log.");
        }
        return Ok(());
    }

    let entries: Vec<AuditRow> = rows
        .into_iter()
        .map(
            |(ts, account, addr, ch, prov, act, ri, rn, err, dur)| AuditRow {
                timestamp: ts,
                account_name: account,
                address: addr,
                chain: ch,
                provider: prov,
                action: act,
                records_in: ri,
                records_new: rn,
                error: err,
                duration_ms: dur,
            },
        )
        .collect();

    if opts.json {
        println!("[");
        for (i, e) in entries.iter().enumerate() {
            let comma = if i < entries.len() - 1 { "," } else { "" };
            println!(
                "  {{\"timestamp\":\"{}\",\"wallet\":\"{}\",\"address\":\"{}\",\
                 \"chain\":\"{}\",\"provider\":\"{}\",\"action\":\"{}\",\
                 \"records_in\":{},\"records_new\":{},\"error\":{},\"duration_ms\":{}}}{}",
                e.timestamp,
                e.account_name,
                e.address,
                e.chain,
                e.provider,
                e.action,
                e.records_in.map_or("null".to_string(), |v| v.to_string()),
                e.records_new.map_or("null".to_string(), |v| v.to_string()),
                e.error
                    .as_deref()
                    .map_or("null".to_string(), |s| format!("\"{}\"", s)),
                e.duration_ms.map_or("null".to_string(), |v| v.to_string()),
                comma
            );
        }
        println!("]");
        return Ok(());
    }

    // Human-readable table
    let ok_count = entries.iter().filter(|e| e.error.is_none()).count();
    let err_count = entries.iter().filter(|e| e.error.is_some()).count();

    println!("{}", "Sync Audit Log".bold());
    println!(
        "  {} total  •  {} ok  •  {}",
        entries.len(),
        ok_count,
        if err_count > 0 {
            format!("{} errors", err_count).red().to_string()
        } else {
            "0 errors".to_string()
        }
    );
    println!();

    // Column widths
    let ts_w = 19;
    let wallet_w = 16;
    let addr_w = 16;
    let chain_w = 8;
    let prov_w = 14;
    let act_w = 14;

    println!(
        "  {:<ts_w$}  {:<wallet_w$}  {:<addr_w$}  {:<chain_w$}  {:<prov_w$}  {:<act_w$}  {:>6}  {:>6}  {:>7}  {}",
        "Timestamp".dimmed(),
        "Wallet".dimmed(),
        "Address".dimmed(),
        "Chain".dimmed(),
        "Provider".dimmed(),
        "Action".dimmed(),
        "In".dimmed(),
        "New".dimmed(),
        "ms".dimmed(),
        "Status".dimmed(),
    );
    println!("  {}", "-".repeat(110).dimmed());

    for e in &entries {
        let ts = &e.timestamp[..ts_w.min(e.timestamp.len())];
        let wallet_short = truncate(&e.account_name, wallet_w);
        let addr_short = truncate(&e.address, addr_w);
        let chain_short = truncate(&e.chain, chain_w);
        let prov_short = truncate(&e.provider, prov_w);
        let act_short = truncate(&e.action, act_w);
        let ri = e
            .records_in
            .map_or("  -".to_string(), |v| format!("{:>6}", v));
        let rn = e
            .records_new
            .map_or("  -".to_string(), |v| format!("{:>6}", v));
        let dur = e
            .duration_ms
            .map_or("      -".to_string(), |v| format!("{:>7}", v));

        let status = if let Some(ref err) = e.error {
            format!("✗ {}", truncate(err, 40)).red().to_string()
        } else {
            "✓".green().to_string()
        };

        println!(
            "  {:<ts_w$}  {:<wallet_w$}  {:<addr_w$}  {:<chain_w$}  {:<prov_w$}  {:<act_w$}  {}  {}  {}  {}",
            ts,
            wallet_short,
            addr_short,
            chain_short,
            prov_short,
            act_short,
            ri,
            rn,
            dur,
            status
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// audit coverage
// ---------------------------------------------------------------------------

async fn handle_audit_coverage(
    wallet: Option<String>,
    chain: Option<String>,
    pool: &SqlitePool,
    opts: &GlobalOptions,
) -> Result<()> {
    // Join wallet_sync_state (left join — include unsynced addresses too)
    let rows: Vec<(
        String,         // account_name
        String,         // blockchain
        String,         // address
        Option<i64>,    // last_block
        Option<String>, // last_sync_at
    )> = sqlx::query_as(
        "SELECT
            COALESCE(a.name, wa.account_id) AS account_name,
            wa.blockchain,
            wa.address,
            wss.last_block,
            wss.last_sync_at
         FROM wallet_addresses wa
         LEFT JOIN accounts a ON a.id = wa.account_id
         LEFT JOIN wallet_sync_state wss ON wss.address = wa.address AND wss.chain = wa.blockchain
         WHERE (? IS NULL OR a.name = ? OR wa.account_id = ?)
           AND (? IS NULL OR wa.blockchain = ?)
         ORDER BY a.name, wa.blockchain, wa.address",
    )
    .bind(&wallet)
    .bind(&wallet)
    .bind(&wallet)
    .bind(&chain)
    .bind(&chain)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        if !opts.quiet {
            println!("No wallet addresses found.");
            println!("Add one with: cryptofolio wallet add");
        }
        return Ok(());
    }

    let synced = rows.iter().filter(|r| r.4.is_some()).count();
    let never_synced = rows.len() - synced;

    if opts.json {
        println!("[");
        for (i, (acct, bc, addr, block, last_at)) in rows.iter().enumerate() {
            let comma = if i < rows.len() - 1 { "," } else { "" };
            println!(
                "  {{\"wallet\":\"{}\",\"chain\":\"{}\",\"address\":\"{}\",\
                 \"last_block\":{},\"last_sync_at\":{}}}{}",
                acct,
                bc,
                addr,
                block.map_or("null".to_string(), |v| v.to_string()),
                last_at
                    .as_deref()
                    .map_or("null".to_string(), |s| format!("\"{}\"", s)),
                comma
            );
        }
        println!("]");
        return Ok(());
    }

    println!("{}", "Sync Coverage".bold());
    println!(
        "  {} addresses  •  {} synced  •  {}",
        rows.len(),
        synced,
        if never_synced > 0 {
            format!("{} never synced", never_synced)
                .yellow()
                .to_string()
        } else {
            "all synced".green().to_string()
        }
    );
    println!();

    let wallet_w = 16;
    let chain_w = 9;
    let addr_w = 20;
    let block_w = 10;

    println!(
        "  {:<wallet_w$}  {:<chain_w$}  {:<addr_w$}  {:>block_w$}  {}",
        "Wallet".dimmed(),
        "Chain".dimmed(),
        "Address".dimmed(),
        "Last Block".dimmed(),
        "Last Synced".dimmed(),
    );
    println!("  {}", "-".repeat(75).dimmed());

    for (acct, bc, addr, block, last_at) in &rows {
        let wallet_short = truncate(acct, wallet_w);
        let addr_short = truncate(addr, addr_w);
        let block_str = block.map_or("-".to_string(), |v| v.to_string());
        let sync_str = last_at
            .as_deref()
            .map(|s| s.get(..16).unwrap_or(s).to_string())
            .unwrap_or_else(|| "never".yellow().to_string());

        println!(
            "  {:<wallet_w$}  {:<chain_w$}  {:<addr_w$}  {:>block_w$}  {}",
            wallet_short, bc, addr_short, block_str, sync_str,
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// audit errors
// ---------------------------------------------------------------------------

async fn handle_audit_errors(limit: i64, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT
            sal.timestamp,
            COALESCE(a.name, sal.account_id) AS account_name,
            sal.address,
            sal.chain,
            sal.provider,
            sal.error
         FROM sync_audit_log sal
         LEFT JOIN accounts a ON a.id = sal.account_id
         WHERE sal.error IS NOT NULL
         ORDER BY sal.timestamp DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        if !opts.quiet {
            println!("{}", "No sync errors found.".green());
        }
        return Ok(());
    }

    if opts.json {
        println!("[");
        for (i, (ts, acct, addr, ch, prov, err)) in rows.iter().enumerate() {
            let comma = if i < rows.len() - 1 { "," } else { "" };
            println!(
                "  {{\"timestamp\":\"{}\",\"wallet\":\"{}\",\"address\":\"{}\",\
                 \"chain\":\"{}\",\"provider\":\"{}\",\"error\":\"{}\"}}{}",
                ts, acct, addr, ch, prov, err, comma
            );
        }
        println!("]");
        return Ok(());
    }

    println!("{}", format!("Sync Errors ({})", rows.len()).bold().red());
    println!();

    for (ts, acct, addr, ch, prov, err) in &rows {
        let ts_short = ts.get(..19).unwrap_or(ts.as_str());
        println!(
            "  {} {} {} {}  {}",
            ts_short.dimmed(),
            format!("[{}]", ch).cyan(),
            acct.bold(),
            addr.dimmed(),
            prov.dimmed(),
        );
        println!("  {} {}", "→".red(), err.red());
        println!();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
