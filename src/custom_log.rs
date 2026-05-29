use anyhow::Context;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;

use crate::ArchiveError;
use crate::cli;

pub fn log_init(cli: &mut cli::Cli) -> anyhow::Result<()> {
    let log_file = FileAppender::builder().build(
        cli.log_file.as_deref_mut().unwrap_or(
            &mut (match &mut cli.command {
                cli::Commands::Copy {
                    source,
                    destination,
                } => {
                    if destination.exists() {
                        *destination =
                            destination.join(source.file_name().ok_or(
                                ArchiveError::InvalidSourceDir(source.display().to_string()),
                            )?);
                    }
                    std::fs::create_dir_all(&destination).with_context(|| {
                        format!(
                            "Impossible de créer le dossier de destination `{}`",
                            destination.display()
                        )
                    })?;
                    destination
                }
                cli::Commands::Prepare { path } => path,
                cli::Commands::Verify { path, .. } => path,
            }
            .join("archivage-historique.log")),
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

        builder.build({
            let mut root_builder = log4rs::config::Root::builder();
            root_builder = root_builder.appender("logfile");
            if !cli.quiet {
                root_builder = root_builder.appender("stderr")
            }
            root_builder.build(log::LevelFilter::Info)
        })?
    };
    log4rs::init_config(config)?;
    Ok(())
}
