use argh::FromArgs;

/// A simple danmaku server.
#[derive(FromArgs)]
pub struct Args {
    /// the address to listen on
    #[argh(option, short = 'a', default = "\"127.0.0.1\".to_string()")]
    pub address: String,

    /// the port to listen on
    #[argh(option, short = 'p', default = "9000")]
    pub port: u16,

    /// the number of threads to use
    #[argh(option, short = 't', default = "num_cpus::get()")]
    pub threads: usize,

    /// verbose logging
    #[argh(switch, short = 'v')]
    pub verbose: bool,

    /// show version information and exit
    #[argh(switch, short = 'V')]
    pub version: bool,
}
