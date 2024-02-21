use questdb::ingress::{
    Buffer, CertificateAuthority, ColumnName, SenderBuilder, TableName, TimestampNanos, Tls,
};
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};

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

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Protocol {
    Http,
    Tcp,
}

/// Simulate traffic to QuestDB over ILP/HTTP or ILP/TCP.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Protocol to use for sending data.
    #[clap(long, default_value = "http")]
    #[arg(value_enum)]
    protocol: Protocol,

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

    /// TCP auth, format: `key_id/priv_key/pub_key_x/pub_key_y`
    #[clap(long)]
    tcp_auth: Option<String>,

    /// Enable TLS for the connection.
    #[clap(long, action = clap::ArgAction::SetTrue)]
    tls: bool,

    /// Frequency at which to print stats.
    /// E.g. if `stats_frequency` is 10, then stats will be printed every 10 requests.
    #[clap(long, default_value = "10")]
    stats_frequency: usize,
}

fn main() -> anyhow::Result<()> {
    let args = CommandArgs::parse();

    let tables = if args.table_count == 1 {
        vec![args.table_name.clone()]
    } else {
        (0..args.table_count)
            .map(|i| format!("{}_{}", args.table_name, i))
            .collect()
    };
    let tables = tables
        .iter()
        .map(|s| TableName::new(s.as_str()).unwrap())
        .collect::<Vec<_>>();

    let symbols = (0..args.symbol_count)
        .map(|i| format!("sym{}", i))
        .collect::<Vec<_>>();
    let symbols = symbols
        .iter()
        .map(|s| (ColumnName::new(s.as_str()).unwrap(), s.as_str()))
        .collect::<Vec<_>>();

    let floats = (0..args.float_count)
        .map(|i| format!("float{}", i))
        .collect::<Vec<_>>();
    let floats = floats
        .iter()
        .enumerate()
        .map(|(i, s)| (ColumnName::new(s.as_str()).unwrap(), i as f64))
        .collect::<Vec<_>>();

    let mut builder = SenderBuilder::new(&args.host, args.port);

    match args.protocol {
        Protocol::Http => {
            builder = builder.http();
            if let Some(token) = args.oauth_token.as_deref() {
                builder = builder.token_auth(token);
            } else {
                builder = builder.basic_auth(&args.basic_auth_user, &args.basic_auth_password);
            }
        }
        Protocol::Tcp => {
            if let Some(auth) = args.tcp_auth.as_ref() {
                let parts = auth.split('/').collect::<Vec<_>>();
                if parts.len() != 4 {
                    return Err(anyhow::anyhow!(
                        "Invalid tcp-auth format, must be `key_id/priv_key/pub_key_x/pub_key_y`"
                    ));
                }
                eprintln!(
                    "Using TCP auth: key_id: {}, priv_key: {}, pub_key_x: {}, pub_key_y: {}",
                    parts[0], parts[1], parts[2], parts[3]
                );
                builder = builder.auth(parts[0], parts[1], parts[2], parts[3]);
            }
        }
    }

    // Apply TLS configuration based on the tls flag
    if args.tls {
        builder = builder.tls(Tls::Enabled(CertificateAuthority::WebpkiRoots));
    }

    let mut sender = builder.connect()?;

    let mut total_sent_rows = 0usize;
    let mut total_sent_bytes = 0usize;
    let mut request_index = 0usize;
    let mut buffer = Buffer::new();
    let begin = Instant::now();
    let mut last_sent = Instant::now() - (2 * args.send_interval);
    loop {
        let to_sleep = (last_sent + args.send_interval).saturating_duration_since(Instant::now());
        if to_sleep > Duration::from_secs(0) {
            std::thread::sleep(to_sleep);
        }
        last_sent = Instant::now();

        for table in tables.iter() {
            write_request(
                &mut buffer,
                args.rows_per_request,
                *table,
                &symbols,
                &floats,
            )?;
        }

        total_sent_rows += buffer.row_count();
        total_sent_bytes += buffer.len();
        sender.flush(&mut buffer)?;

        if request_index != 0 && request_index % args.stats_frequency == 0 {
            let tot_elapsed = begin.elapsed();
            let throughput_rows = total_sent_rows as f64 / tot_elapsed.as_secs_f64();
            let throughput_bytes = total_sent_bytes as f64 / tot_elapsed.as_secs_f64();
            eprintln!(
                "\n[{}] Sent {} rows, {} bytes, {:.2} rows/s, {:.2} bytes/s",
                request_index, total_sent_rows, total_sent_bytes, throughput_rows, throughput_bytes
            );
        }
        if args.stats_frequency <= 20 {
            eprint!(".");
        }

        if begin.elapsed() > args.test_duration {
            break;
        }

        request_index += 1;
    }
    Ok(())
}

fn write_request(
    buffer: &mut Buffer,
    rows_per_request: usize,
    table: TableName,
    symbols: &[(ColumnName, &str)],
    floats: &[(ColumnName, f64)],
) -> anyhow::Result<()> {
    for _r in 0..rows_per_request {
        buffer.table(table)?;
        for (col_name, sym_value) in symbols {
            buffer.symbol(*col_name, sym_value)?;
        }
        for (col_name, float_value) in floats {
            buffer.column_f64(*col_name, *float_value)?;
        }
        buffer.at(TimestampNanos::now())?;
    }
    Ok(())
}
