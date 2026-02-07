use sqlx::SqlitePool;

use crate::cli::{CategoryCommands, GlobalOptions};
use crate::cli::output::{print_header, print_row, success};
use crate::db::AccountRepository;
use crate::error::Result;

pub async fn handle_category_command(command: CategoryCommands, pool: &SqlitePool, opts: &GlobalOptions) -> Result<()> {
    let _ = opts; // Will be used for JSON output
    let repo = AccountRepository::new(pool);

    match command {
        CategoryCommands::List => {
            let categories = repo.list_categories().await?;

            if categories.is_empty() {
                println!("No categories configured.");
                return Ok(());
            }

            print_header(&[("ID", 15), ("Name", 20)]);

            for category in categories {
                print_row(&[
                    (&category.id, 15),
                    (&category.name, 20),
                ]);
            }
        }

        CategoryCommands::Add { name } => {
            let id = name.to_lowercase().replace(' ', "-");
            repo.create_category(&id, &name).await?;
            success(&format!("Category '{}' created", name));
        }

        CategoryCommands::Rename { old_name, new_name } => {
            repo.rename_category(&old_name, &new_name).await?;
            success(&format!("Category renamed from '{}' to '{}'", old_name, new_name));
        }

        CategoryCommands::Remove { name, yes } => {
            if !yes {
                println!("This will delete category '{}'.", name);
                print!("Are you sure? [y/N] ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            repo.delete_category(&name).await?;
            success(&format!("Category '{}' removed", name));
        }
    }

    Ok(())
}
