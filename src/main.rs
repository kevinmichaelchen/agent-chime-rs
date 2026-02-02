use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = agent_chime::cli::Cli::parse();
    agent_chime::run(cli)
}
