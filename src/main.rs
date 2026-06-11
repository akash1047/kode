use kode::{cli, commands};

#[tokio::main]
async fn main() {
    let cli = cli::parse();

    match cli.command {
        None => commands::scan::run_default(&cli.path).await,
        Some(cli::Command::Info { path }) => commands::info::run(&path),
        Some(cli::Command::Scan { path }) => commands::scan::run_default(&path).await,
        Some(cli::Command::Chat { path, message, model }) => {
            commands::chat::run(&path, message, model).await
        }
        Some(cli::Command::Cache { op }) => commands::cache::run(op),
        Some(cli::Command::Mcp { op }) => match op {
            cli::McpOp::Init { path, preset } => commands::mcp::init(&path, preset),
            cli::McpOp::Serve { path, transport, port } => {
                commands::mcp::serve(&path, transport, port).await
            }
        },
        Some(cli::Command::Config { op }) => match op {
            cli::ConfigOp::Init => commands::config::init(),
            cli::ConfigOp::Show => commands::config::show(),
            cli::ConfigOp::Set { key, value } => commands::config::set(&key, &value),
            cli::ConfigOp::Get { key } => commands::config::get(&key),
        },
    }
}
