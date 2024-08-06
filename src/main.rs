use time_tally::args::parse_args;
use time_tally::{run_api_server, run_metrics_server, setup_tracing_subscriber};
use tokio::signal;

#[tokio::main]
async fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments: {}", e);
            std::process::exit(1);
        }
    };

    setup_tracing_subscriber(args.subscriber, args.verbose);

    run_api_server(args.api_network.to_string(), args.api_port.to_string()).await;

    if args.metrics {
        run_metrics_server(
            args.metrics_network.to_string(),
            args.metrics_port.to_string(),
        )
        .await;
    }
    signal::ctrl_c().await.unwrap();
}

/*
Areas for Improvement:


Error Handling:

Implement custom error types for more granular error handling and better error messages.



Input Validation:

Add more robust input validation for query parameters.


Security:

Add security headers middleware.
Implement rate limiting to prevent abuse.


Performance:

Consider adding caching for frequently accessed data.
Implement database pooling if you plan to add a database.


Modularity:

As the project grows, consider splitting the API handlers into separate modules.


Graceful Shutdown:

Implement graceful shutdown handling for your servers.


Containerization:

Consider adding a Dockerfile for easy deployment and scalability.

*/
