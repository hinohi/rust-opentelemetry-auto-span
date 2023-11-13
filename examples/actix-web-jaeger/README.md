# actix-web + Jaeger example

1. Run MySQL + Jaeger

    ```sh
    docker compose up -d
    ```
2. Run actix-web server

    ```sh
    cargo run
    ```
3. access

    ```sh
    curl 'http://localhost:8081/' 
    curl 'http://localhost:8081/user/1' 
    curl 'http://localhost:8081/user/10' 
    ```
4. Open http://localhost:16686
