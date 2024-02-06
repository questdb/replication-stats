use std::time::{Duration, Instant};
use questdb::ingress::{SenderBuilder, Buffer, TimestampNanos};

use clap::Parser;

fn parse_duration(arg: &str) -> anyhow::Result<Duration> {
    let nanos = match go_parse_duration::parse_duration(arg) {
        Ok(nanos) => nanos as u64,
        Err(go_parse_duration::Error::ParseError(msg)) => return Err(anyhow::anyhow!("{}", msg)),
    };
    Ok(Duration::from_nanos(nanos))
}

/// Simulate traffic to QuestDB over ILP/HTTP
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Hostname of the QuestDB server
    #[clap(long, default_value = "localhost")]
    host: String,

    /// Port number of the QuestDB server
    #[clap(long, default_value = "9000")]
    port: u16,

    /// Name of the table to send data to
    #[clap(long, default_value = "test")]
    table_name: String,

    /// Interval between sending rows
    #[clap(long, default_value = "1s")]
    #[arg(value_parser = parse_duration)]
    send_interval: Duration,

    /// Number of rows per HTTP request
    #[clap(long, default_value_t = 1)]
    rows_per_request: usize,

    /// Duration of the test
    #[clap(long, default_value = "10m")]
    #[arg(value_parser = parse_duration)]
    test_duration: Duration,
}

fn main() -> anyhow::Result<()> {
    let args = CommandArgs::parse();
    let mut sender = SenderBuilder::new("localhost", 9000)
        .http()
        .basic_auth("admin", "quest")
        .connect()?;
    let mut buffer = Buffer::new();
    let begin = Instant::now();
    let mut last_sent = Instant::now() - (2 * args.send_interval);
    loop {
        let to_sleep = (last_sent + args.send_interval).saturating_duration_since(Instant::now());
        if to_sleep > Duration::from_secs(0) {
            std::thread::sleep(to_sleep);
        }
        last_sent = Instant::now();
        for _ in 0..args.rows_per_request {
            write_row(&args, &mut buffer)?;
        }
        sender.flush(&mut buffer)?;
        eprint!(".");

        if begin.elapsed() > args.test_duration {
            break;
        }
    }
    Ok(())
}

fn write_row(args: &CommandArgs, buffer: &mut Buffer) -> anyhow::Result<()> {
    buffer.table(&args.table_name)?
        .symbol("sym0", "sym0")?
        .symbol("sym1", "sym1")?
        .symbol("sym2", "sym2")?
        .symbol("sym3", "sym3")?
        .symbol("sym4", "sym4")?
        .symbol("sym5", "sym5")?
        .symbol("sym6", "sym6")?
        .symbol("sym7", "sym7")?
        .symbol("sym8", "sym8")?
        .symbol("sym9", "sym9")?
        .column_f64("f0", 0.0)?
        .column_f64("f1", 1.0)?
        .column_f64("f2", 2.0)?
        .column_f64("f3", 3.0)?
        .column_f64("f4", 4.0)?
        .column_f64("f5", 5.0)?
        .column_f64("f6", 6.0)?
        .column_f64("f7", 7.0)?
        .column_f64("f8", 8.0)?
        .column_f64("f9", 9.0)?
        .column_i64("i0", 0)?
        .column_i64("i1", 1)?
        .column_i64("i2", 2)?
        .column_i64("i3", 3)?
        .column_i64("i4", 4)?
        .column_i64("i5", 5)?
        .column_i64("i6", 6)?
        .column_i64("i7", 7)?
        .column_i64("i8", 8)?
        .column_i64("i9", 9)?
        .at(TimestampNanos::now())?;
    Ok(())
}
