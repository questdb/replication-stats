# ILP HTTP Traffic Generator

## Build and see usage

```
cargo run --release -- --help
```

```
    Finished release [optimized] target(s) in 0.03s
     Running `target/release/ilp-http-traffic-generator --help`
```

```
Simulate traffic to QuestDB over ILP/HTTP

Usage: ilp-http-traffic-generator [OPTIONS]

Options:
      --host <HOST>
          Hostname of the QuestDB server [default: localhost]
      --port <PORT>
          Port number of the QuestDB server [default: 9000]
      --table-name <TABLE_NAME>
          Name of the table to send data to. This is a prefix if `table_count` is greater than 1 [default: test]
      --send-interval <SEND_INTERVAL>
          Interval between sending requests [default: 1s]
      --table-count <TABLE_COUNT>
          Number of tables to send data to in each request. Each table name is `{table_name}_{i}`, unless `table_count` is 1 [default: 1]
      --rows-per-request <ROWS_PER_REQUEST>
          Number of rows per table per HTTP request. I.e. if `table_count` is 2, and `rows_per_request` is 3, then 6 rows will be sent in each request [default: 1]
      --symbol-count <SYMBOL_COUNT>
          Number of symbol columns in each row [default: 10]
      --float-count <FLOAT_COUNT>
          Number of float columns in each row [default: 10]
      --test-duration <TEST_DURATION>
          Duration of the test. E.g. `10s`, `5m`, `1h`, `2h30m`, etc [default: 10m]
  -h, --help
          Print help
  -V, --version
          Print version
```

