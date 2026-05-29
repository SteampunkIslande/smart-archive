use anyhow::Context;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;

use crate::cli;

pub fn log_init(cli: &cli::Cli) -> anyhow::Result<()> {
    let log_file = FileAppender::builder().build(
        cli.log_file.as_ref().unwrap_or(
            &match &cli.command {
                &cli::Commands::Copy {
                    source: _,
                    ref destination,
                } => {
                    std::fs::create_dir_all(destination).with_context(|| {
                        format!(
                            "Impossible de créer le dossier de destination `{}`",
                            destination.display()
                        )
                    })?;
                    destination
                }
                &cli::Commands::Prepare { ref path } => path,
                &cli::Commands::Verify { ref path, .. } => path,
            }
            .join("archivage-historique.log"),
        ),
    )?;
    let stderr_file = ConsoleAppender::builder().target(Target::Stderr).build();
    let config = {
        let mut builder = log4rs::Config::builder();
        builder = builder
            .appender(log4rs::config::Appender::builder().build("logfile", Box::new(log_file)));

        if !cli.quiet {
            builder = builder.appender(
                log4rs::config::Appender::builder().build("stderr", Box::new(stderr_file)),
            );
        }

        builder.build(
            log4rs::config::Root::builder()
                .appender("logfile")
                .build(log::LevelFilter::Info),
        )?
    };
    log4rs::init_config(config)?;
    Ok(())
}
