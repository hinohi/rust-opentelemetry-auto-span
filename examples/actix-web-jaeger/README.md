# actix-web + Jaeger example

1. Run MySQL + Jaeger

    ```sh
    cd ../
    docker compose up -d
    cd actix-web-jaeger
    ```
2. Run actix-web server

    ```sh 
    cargo run
    ```
3. Access

    ```sh
    curl 'http://localhost:8081/'
    curl 'http://localhost:8081/user/1'
    curl 'http://localhost:8081/user/10'
    curl 'http://localhost:8081/awc'
    ```
4. See http://localhost:16686
