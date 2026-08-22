# article-server

## Stress Test

1. install oha
    ```shell
    cargo install oha
    ```
2. run article-server with release mode
    ```shell
    cargo run --release --manifest-path examples/article-server/Cargo.toml
    ```
3. Stress Test
    ```shell
    oha -z 10s -c 1 --no-tui "http://127.0.0.1:9090/articles/1"
    ```
   ```shell
   oha -z 30s -c 10 --no-tui "http://127.0.0.1:9090/articles/1"
   ```
    ```shell
    oha -z 30s -c 50 --no-tui "http://127.0.0.1:9090/articles/1"
    ```
    ```shell
    oha -z 30s -c 100 --no-tui "http://127.0.0.1:9090/articles/1"
    ```
    ```shell
    oha -z 30s -c 1000 --no-tui "http://127.0.0.1:9090/articles/1"
    ```

   ```shell
   oha -z 30s -c 100 --no-tui "http://127.0.0.1:9090/articles?page=1&size=10"
   ```
   
   ```shell
   oha -z 30s -c 10 --no-tui `
      -m POST `
      -H "Content-Type: application/json" `
      -d '{"name":"Load Test","content":"Created by oha"}' `
      "http://127.0.0.1:9090/articles"
   ```
   
   ```shell
   oha -z 60s -c 50 -q 500 --latency-correction --no-tui "http://127.0.0.1:9090/articles?page=1&size=10"
   ```