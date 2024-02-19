# replication-stats

## In this repo

This repo contains:

1. [ilp-http-traffic-generator](ilp-http-traffic-generator/README.md): Send synthetic HTTP traffic.
2. [net-traffic-capture](net-traffic-capture/README.md): Capture network packet size stats for specific localhost ports.
3. [analisys](analisys/plot_net_activity.ipynb): Python notebook to analyse [net-traffic-capture](net-traffic-capture/README.md)'s binary capture dumps.
4. [captures](captures/): A few previous captures.

## Profiling replication network usage

The analisys for https://questdb.io/docs/guides/replication-tuning/ has been done using this repo.

It was performed by starting a mock S3 server using [s3s-fs](https://github.com/Nugine/s3s).

To install `s3s-fs`:

```
cargo install s3s-fs --features binary --locked
```

Then, in an empty directory drop `run.sh`

```
export RUST_LOG="s3s=info,s3s_fs=info"
mkdir -p test-bucket
s3s-fs --host 127.0.0.1 --port 10101 --access-key ANOTREAL --secret-key notrealrnrELgWzOk3IfjzDKtFBhDby .
```

and run it.
```
$ ./run.sh &
  2024-02-19T14:59:41.769575Z  INFO s3s_fs: server is running at http://127.0.0.1:10101
    at /home/adam/.cargo/registry/src/index.crates.io-6f17d22bba15001f/s3s-fs-0.8.1/src/main.rs:104
```

You can then configure replication on the primary instance with the following `replication.object.store` in the enterprise DB's `server.conf`:

```
replication.object.store=s3::root=test/root;bucket=test-bucket;region=us-east-1;endpoint=http://localhost:10101;access_key_id=ANOTREAL;secret_access_key=notrealrnrELgWzOk3IfjzDKtFBhDby;
```

Then tune the rest of the tests as appropriate.

If you want to trace network activity, be sure to begin network capture against ports 9000 (for ILP/HTTP) and 10101 (for the mock S3 service).
