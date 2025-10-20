//! Transaction example
//!
//! This example demonstrates transaction management including:
//! - Beginning transactions
//! - Committing changes
//! - Rolling back on errors
//! - ACID properties
//!
//! Run with: cargo run --example transactions

use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Rust Database System - Transaction Example ===\n");

    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;

    // Create accounts table
    println!("1. Setting up accounts table...");
    db.execute(
        "CREATE TABLE accounts (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            balance REAL NOT NULL CHECK(balance >= 0)
        )",
    )
    .await?;

    // Insert initial data using parameterized queries (prevents SQL injection)
    let accounts = vec![
        (1, "Alice", 1000.0),
        (2, "Bob", 500.0),
        (3, "Charlie", 750.0),
    ];

    for (id, name, balance) in accounts {
        db.execute_with_params(
            "INSERT INTO accounts (id, name, balance) VALUES (?, ?, ?)",
            &[id.into(), name.into(), balance.into()],
        )
        .await?;
    }
    println!("   ✓ Accounts created\n");

    // Display initial balances
    print_balances(&mut db).await?;

    // Example 1: Successful transaction
    println!("\n2. Example 1: Successful transaction (Alice -> Bob: $100)");
    db.begin_transaction().await?;
    println!("   ✓ Transaction started");

    match transfer(&mut db, 1, 2, 100.0).await {
        Ok(_) => {
            db.commit().await?;
            println!("   ✓ Transaction committed");
        }
        Err(e) => {
            db.rollback().await?;
            println!("   ✗ Transaction rolled back: {}", e);
        }
    }

    print_balances(&mut db).await?;

    // Example 2: Failed transaction (insufficient funds)
    println!("\n3. Example 2: Failed transaction (Bob -> Alice: $1000 - insufficient funds)");
    db.begin_transaction().await?;
    println!("   ✓ Transaction started");

    match transfer(&mut db, 2, 1, 1000.0).await {
        Ok(_) => {
            db.commit().await?;
            println!("   ✓ Transaction committed");
        }
        Err(e) => {
            db.rollback().await?;
            println!("   ✗ Transaction rolled back: {}", e);
        }
    }

    print_balances(&mut db).await?;

    // Example 3: Multiple operations in one transaction
    println!("\n4. Example 3: Multiple operations in one transaction");
    db.begin_transaction().await?;
    println!("   ✓ Transaction started");

    let operations = vec![
        ("Alice", "Bob", 50.0),
        ("Bob", "Charlie", 100.0),
        ("Charlie", "Alice", 25.0),
    ];

    let mut success = true;
    for (from, to, amount) in operations {
        print!(
            "   - Transferring ${:.2} from {} to {}... ",
            amount, from, to
        );

        let from_id = get_account_id(&mut db, from).await?;
        let to_id = get_account_id(&mut db, to).await?;

        match transfer(&mut db, from_id, to_id, amount).await {
            Ok(_) => println!("✓"),
            Err(e) => {
                println!("✗ ({})", e);
                success = false;
                break;
            }
        }
    }

    if success {
        db.commit().await?;
        println!("   ✓ All operations committed");
    } else {
        db.rollback().await?;
        println!("   ✗ All operations rolled back");
    }

    print_balances(&mut db).await?;

    // Example 4: Atomicity test
    println!("\n5. Example 4: Atomicity test (transfer larger than balance)");
    println!("   Before transaction:");
    print_balances(&mut db).await?;

    db.begin_transaction().await?;
    println!("\n   ✓ Transaction started");

    // Try to transfer more than available
    match transfer(&mut db, 2, 1, 10000.0).await {
        Ok(_) => {
            db.commit().await?;
            println!("   ✓ Transaction committed");
        }
        Err(e) => {
            db.rollback().await?;
            println!("   ✗ Transaction rolled back: {}", e);
        }
    }

    println!("\n   After rollback:");
    print_balances(&mut db).await?;

    println!("\n=== Example completed successfully! ===");

    Ok(())
}

/// Transfer money between accounts
async fn transfer(db: &mut impl Database, from_id: i32, to_id: i32, amount: f64) -> Result<()> {
    // Withdraw from source account
    let affected = db
        .execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ? AND balance >= ?",
            &[amount.into(), from_id.into(), amount.into()],
        )
        .await?;

    if affected == 0 {
        return Err(DatabaseError::query(
            "Insufficient funds or invalid account",
        ));
    }

    // Deposit to destination account
    db.execute_with_params(
        "UPDATE accounts SET balance = balance + ? WHERE id = ?",
        &[amount.into(), to_id.into()],
    )
    .await?;

    Ok(())
}

/// Get account ID by name
async fn get_account_id(db: &mut impl Database, name: &str) -> Result<i32> {
    let results = db
        .query_with_params("SELECT id FROM accounts WHERE name = ?", &[name.into()])
        .await?;

    if results.is_empty() {
        return Err(DatabaseError::query("Account not found"));
    }

    results
        .first()
        .and_then(|row| row.get("id"))
        .and_then(|v| v.as_int())
        .ok_or_else(|| DatabaseError::QueryError("Failed to get account id".to_string()))
}

/// Print all account balances
async fn print_balances(db: &mut impl Database) -> Result<()> {
    let results = db
        .query("SELECT name, balance FROM accounts ORDER BY id")
        .await?;

    println!("   Current balances:");
    for row in results {
        let name = row
            .get("name")
            .ok_or_else(|| DatabaseError::ColumnNotFound("name".to_string()))?
            .as_string();
        let balance = row
            .get("balance")
            .and_then(|v| v.as_double())
            .ok_or_else(|| DatabaseError::ColumnNotFound("balance".to_string()))?;
        println!("   - {}: ${:.2}", name, balance);
    }

    Ok(())
}
