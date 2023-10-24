use std::net::{IpAddr, TcpListener};

use easy_parallel::Parallel;
use eyre::Result;
use smol::{channel, future, prelude::*, Async, Executor};

use philostone::listen;

mod cli;

pub(crate) mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Run a future on multiple threads.
///
/// # Panics
/// Panics if the number of threads is zero, or if the future panics.
fn run_multi_thread<T>(num_threads: usize, future: impl Future<Output = T>) -> T {
    if num_threads == 0 {
        panic!("number of threads must be greater than zero");
    }

    let ex = Executor::new();
    let (signal, shutdown) = channel::unbounded::<()>();

    Parallel::new()
        .each(0..num_threads - 1, |_| {
            future::block_on(ex.run(async {
                let _ = shutdown.recv().await;
            }))
        })
        .finish_in::<_, _, ()>(|| {
            future::block_on(async {
                let res = future.await;
                drop(signal);
                res
            })
        })
        .1
}

fn main() -> Result<()> {
    let args: cli::Args = argh::from_env();
    if args.version {
        println!(
            "{name} v{version}, {target}, built {commit}, with {compiler}, on {datetime}, {profile} profile, optlevel={optlevel}, features: [{features}]",
            name = built_info::PKG_NAME,
            version = built_info::PKG_VERSION,
            target = built_info::TARGET,
            commit = built_info::GIT_COMMIT_HASH.unwrap_or("unknown"),
            compiler = built_info::RUSTC_VERSION,
            datetime = built_info::BUILT_TIME_UTC,
            profile = built_info::PROFILE,
            optlevel = built_info::OPT_LEVEL,
            features = built_info::FEATURES_STR,
        );

        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    run_multi_thread(args.threads, async {
        let listener = Async::<TcpListener>::bind((args.address.parse::<IpAddr>()?, args.port))?;
        listen(listener).await?;
        Ok(())
    })
}
