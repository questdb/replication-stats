# ILP Traffic Generator

## Build and see usage

```
cargo run --release -- --help
```

```
    Finished release [optimized] target(s) in 0.03s
     Running `target/release/ilp-http-traffic-generator --help`
```

```
Simulate traffic to QuestDB over ILP/HTTP or ILP/TCP

Usage: ilp-http-traffic-generator [OPTIONS]

Options:
      --protocol <PROTOCOL>
          Protocol to use for sending data [default: http] [possible values: http, tcp]
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
      --basic-auth-user <BASIC_AUTH_USER>
          Basic auth user. It will be used unless oauth-token is set [default: admin]
      --basic-auth-password <BASIC_AUTH_PASSWORD>
          Basic auth password. It will be used unless oauth-token is set [default: quest]
      --oauth-token <OAUTH_TOKEN>
          Oauth token. If this is set, no basic auth info will be sent
      --tcp-auth <TCP_AUTH>
          TCP auth, format: `key_id/priv_key/pub_key_x/pub_key_y`
      --tls
          Enable TLS for the connection
      --stats-frequency <STATS_FREQUENCY>
          Frequency at which to print stats. E.g. if `stats_frequency` is 10, then stats will be printed every 10 requests [default: 10]
  -h, --help
          Print help
  -V, --version
          Print version
```

