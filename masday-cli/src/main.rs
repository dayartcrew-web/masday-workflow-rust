//! CLI entry point

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "masday")]
#[command(about = "Masday workflow CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup the project
    Setup,
    /// Run database migrations
    DbMigrate,
    /// Start the API server
    Serve,
    /// Show workflow status
    Status {
        /// Workflow ID
        id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => {
            println!("Setting up Masday project...");
            // Placeholder: would run setup logic
            Ok(())
        }
        Commands::DbMigrate => {
            println!("Running database migrations...");
            // Placeholder: would run migrations
            Ok(())
        }
        Commands::Serve => {
            println!("Starting API server...");
            // Placeholder: would start API server
            Ok(())
        }
        Commands::Status { id } => {
            match id {
                Some(id) => println!("Workflow status for: {}", id),
                None => println!("Listing all workflows..."),
            }
            // Placeholder: would fetch and display status
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_main() {
        // Placeholder test
    }
}
