use std::time::{Duration, Instant};
use questdb::ingress::{SenderBuilder, CertificateAuthority, Tls, Buffer, TimestampNanos};

use clap::Parser;

fn parse_duration(arg: &str) -> anyhow::Result<Duration> {
    let nanos = match go_parse_duration::parse_duration(arg) {
        Ok(nanos) => nanos as u64,
        Err(go_parse_duration::Error::ParseError(msg)) => return Err(anyhow::anyhow!("{}", msg)),
    };
    Ok(Duration::from_nanos(nanos))
}

fn at_least_one(s: &str) -> Result<usize, String> {
    let n = s.parse::<usize>().map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("cannot be zero".to_string());
    }
    Ok(n)
}

/// Simulate traffic to QuestDB over ILP/HTTP.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Hostname of the QuestDB server.
    #[clap(long, default_value = "localhost")]
    host: String,

    /// Port number of the QuestDB server.
    #[clap(long, default_value = "9000")]
    port: u16,

    /// Name of the table to send data to.
    /// This is a prefix if `table_count` is greater than 1.
    #[clap(long, default_value = "test")]
    table_name: String,

    /// Interval between sending requests.
    #[clap(long, default_value = "1s")]
    #[arg(value_parser = parse_duration)]
    send_interval: Duration,

    /// Number of tables to send data to in each request.
    /// Each table name is `{table_name}_{i}`, unless `table_count` is 1.
    #[clap(long, default_value_t = 1, value_parser=at_least_one)]
    table_count: usize,

    /// Number of rows per table per HTTP request.
    /// I.e. if `table_count` is 2, and `rows_per_request` is 3, then 6 rows will be sent in each request.
    #[clap(long, default_value_t = 1)]
    rows_per_request: usize,

    /// Number of symbol columns in each row.
    #[clap(long, default_value = "10")]
    symbol_count: usize,

    /// Number of float columns in each row.
    #[clap(long, default_value = "10")]
    float_count: usize,

    /// Duration of the test. E.g. `10s`, `5m`, `1h`, `2h30m`, etc.
    #[clap(long, default_value = "10m")]
    #[arg(value_parser = parse_duration)]
    test_duration: Duration,

    /// Basic auth user.
    /// It will be used unless oauth-token is set.
    #[clap(long, default_value = "admin")]
    basic_auth_user: String,

    /// Basic auth password.
    /// It will be used unless oauth-token is set.
    #[clap(long, default_value = "quest")]
    basic_auth_password: String,

    /// Oauth token.
    /// If this is set, no basic auth info will be sent.
    #[clap(long)]
    oauth_token: Option<String>,

    /// Enable TLS for the connection.
    #[clap(long, action = clap::ArgAction::SetTrue)]
    tls: bool,
}

fn main() -> anyhow::Result<()> {
    let args = CommandArgs::parse();
    let mut builder = SenderBuilder::new(&args.host, args.port.clone()).http();

    if let Some(token) = args.oauth_token.as_deref() {
        builder = builder.token_auth(token);
    } else {
        builder = builder.basic_auth(&args.basic_auth_user, &args.basic_auth_password);
    }

    // Apply TLS configuration based on the tls flag
    if args.tls {
        builder = builder.tls(Tls::Enabled(CertificateAuthority::WebpkiRoots));
    }

    let mut sender = builder.connect()?;

    let mut buffer = Buffer::new();
    let begin = Instant::now();
    let mut last_sent = Instant::now() - (2 * args.send_interval);
    loop {
        let to_sleep = (last_sent + args.send_interval).saturating_duration_since(Instant::now());
        if to_sleep > Duration::from_secs(0) {
            std::thread::sleep(to_sleep);
        }
        last_sent = Instant::now();

        if args.table_count == 1 {
            write_row(&args.table_name, &args, &mut buffer)?;
        } else {
            for i in 0..args.table_count {
                write_row(&format!("{}_{}", args.table_name, i), &args, &mut buffer)?;
            }
        }

        sender.flush(&mut buffer)?;
        eprint!(".");

        if begin.elapsed() > args.test_duration {
            break;
        }
    }
    Ok(())
}

fn write_row(table: &str, args: &CommandArgs, buffer: &mut Buffer) -> anyhow::Result<()> {
    for _r in 0..args.rows_per_request {
        buffer.table(table)?;
        for i in 0..args.symbol_count {
            buffer.symbol(format!("sym{}", i).as_str(), format!("sym{}", i))?;
        }
        for i in 0..args.float_count {
            buffer.column_f64(format!("f{}", i).as_str(), i as f64)?;
        }
        buffer.at(TimestampNanos::now())?;
    }
    Ok(())
}
