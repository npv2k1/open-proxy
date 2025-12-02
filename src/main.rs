use clap::{Parser, Subcommand};
use open_proxy::{
    database::TodoDatabase,
    models::Todo,
    proxy::{CheckerConfig, ProxyChecker, ProxyParser, ProxyType},
    tui::App,
};
use std::path::PathBuf;
use std::time::Duration;

/// A proxy parser and checker with multi-threading support
#[derive(Parser)]
#[command(name = "open-proxy")]
#[command(about = "A proxy parser and checker with multi-threading support")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Database file path
    #[arg(short, long, default_value = "todo.db")]
    database: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the interactive TUI
    Tui,
    /// List all todos
    List {
        /// Show only completed todos
        #[arg(short, long)]
        completed: bool,
        /// Show only pending todos
        #[arg(short, long)]
        pending: bool,
    },
    /// Add a new todo
    Add {
        /// Todo title
        title: String,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Complete a todo by ID
    Complete {
        /// Todo ID
        id: String,
    },
    /// Delete a todo by ID
    Delete {
        /// Todo ID
        id: String,
    },
    /// Parse proxies from a file
    Parse {
        /// Input file containing proxies
        input: PathBuf,
        /// Output file for parsed proxies
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Proxy type (http, https, socks4, socks5)
        #[arg(short = 't', long, default_value = "http")]
        proxy_type: String,
    },
    /// Check proxies and save results
    Check {
        /// Input file containing proxies
        input: PathBuf,
        /// Output file for good proxies
        #[arg(short, long)]
        good: Option<PathBuf>,
        /// Output file for bad proxies
        #[arg(short, long)]
        bad: Option<PathBuf>,
        /// Proxy type (http, https, socks4, socks5)
        #[arg(short = 't', long, default_value = "http")]
        proxy_type: String,
        /// Number of concurrent threads
        #[arg(short = 'n', long, default_value = "10")]
        threads: usize,
        /// Timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
        /// URL to test proxies against
        #[arg(long, default_value = "http://httpbin.org/ip")]
        test_url: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db = TodoDatabase::new(&cli.database).await?;

    match cli.command {
        Some(Commands::Tui) | None => {
            // Default to TUI mode
            let mut app = App::new(db);
            app.run().await?;
        }
        Some(Commands::List { completed, pending }) => {
            let todos = if completed {
                db.get_todos_by_status(true).await?
            } else if pending {
                db.get_todos_by_status(false).await?
            } else {
                db.get_all_todos().await?
            };

            if todos.is_empty() {
                println!("No todos found.");
            } else {
                for todo in todos {
                    let status = if todo.completed { "✓" } else { "○" };
                    println!("{} {} - {}", status, todo.title, todo.id);
                    if let Some(description) = &todo.description {
                        println!("   {}", description);
                    }
                }
            }
        }
        Some(Commands::Add { title, description }) => {
            let todo = Todo::new(title, description);
            db.create_todo(&todo).await?;
            println!("Todo added: {}", todo.id);
        }
        Some(Commands::Complete { id }) => {
            if let Some(mut todo) = db.get_todo(&id).await? {
                todo.complete();
                db.update_todo(&todo).await?;
                println!("Todo completed: {}", todo.title);
            } else {
                eprintln!("Todo not found: {}", id);
            }
        }
        Some(Commands::Delete { id }) => {
            if let Some(todo) = db.get_todo(&id).await? {
                db.delete_todo(&id).await?;
                println!("Todo deleted: {}", todo.title);
            } else {
                eprintln!("Todo not found: {}", id);
            }
        }
        Some(Commands::Parse {
            input,
            output,
            proxy_type,
        }) => {
            let ptype = parse_proxy_type(&proxy_type)?;
            let proxies = ProxyParser::parse_file(&input, ptype)?;
            
            println!("Parsed {} proxies from {:?}", proxies.len(), input);
            
            if let Some(output_path) = output {
                ProxyParser::save_to_file(&proxies, &output_path, true)?;
                println!("Saved parsed proxies to {:?}", output_path);
            } else {
                for proxy in &proxies {
                    println!("{}", proxy.to_full_string());
                }
            }
        }
        Some(Commands::Check {
            input,
            good,
            bad,
            proxy_type,
            threads,
            timeout,
            test_url,
        }) => {
            let ptype = parse_proxy_type(&proxy_type)?;
            let proxies = ProxyParser::parse_file(&input, ptype)?;
            
            println!("Loaded {} proxies from {:?}", proxies.len(), input);
            println!("Checking with {} threads, timeout: {}s", threads, timeout);
            println!("Test URL: {}", test_url);
            println!();

            let config = CheckerConfig::new()
                .with_concurrency(threads)
                .with_timeout(Duration::from_secs(timeout))
                .with_test_url(test_url);

            let checker = ProxyChecker::with_config(config);
            let (good_results, bad_results) = checker.check_and_separate(proxies).await;

            println!("Results: {} good, {} bad", good_results.len(), bad_results.len());

            // Save good proxies
            if let Some(good_path) = good {
                let good_proxies: Vec<_> = good_results.iter().map(|r| r.proxy.clone()).collect();
                ProxyParser::save_to_file(&good_proxies, &good_path, true)?;
                println!("Saved {} good proxies to {:?}", good_proxies.len(), good_path);
            }

            // Save bad proxies
            if let Some(bad_path) = bad {
                let bad_proxies: Vec<_> = bad_results.iter().map(|r| r.proxy.clone()).collect();
                ProxyParser::save_to_file(&bad_proxies, &bad_path, true)?;
                println!("Saved {} bad proxies to {:?}", bad_proxies.len(), bad_path);
            }

            // Print working proxies with response times
            if !good_results.is_empty() {
                println!("\nWorking proxies:");
                for result in &good_results {
                    if let Some(time) = result.response_time_ms {
                        println!("  {} ({}ms)", result.proxy.to_full_string(), time);
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_proxy_type(s: &str) -> Result<ProxyType, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "http" => Ok(ProxyType::Http),
        "https" => Ok(ProxyType::Https),
        "socks4" => Ok(ProxyType::Socks4),
        "socks5" => Ok(ProxyType::Socks5),
        _ => Err(format!("Invalid proxy type: {}. Use: http, https, socks4, socks5", s).into()),
    }
}
