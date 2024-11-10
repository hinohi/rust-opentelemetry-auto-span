# axum + Jaeger example

1. Run MySQL + Jaeger

    ```sh
    cd ../
    docker compose up -d
    cd axum-jaeger
    ```
2. Run axum-web server

    ```sh
    cargo run
    ```
3. access

    ```sh
    curl 'http://localhost:3000/'
    curl 'http://localhost:3000/user/1'
    curl 'http://localhost:3000/user/10'
    ```
4. Open http://localhost:16686
