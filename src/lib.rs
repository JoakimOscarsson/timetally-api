//! Axum-based API server for calculating work hours.
//!
//! This module provides the core functionality for setting up and running
//! the API server, including tracing, routing, and request handling.
//!
//! # Main components
//!
//! - `setup_tracing_subscriber`: Configures the tracing subscriber for logging.
//! - `run_api_server`: Sets up and runs the main API server.
//! - `run_metrics_server`: Sets up and runs a separate metrics server.
//! - `get_workhours`: Handles requests to calculate work hours.
//!
//!  # Examples
//! ```no_run
//! use time_tally::args::parse_args;
//! use time_tally::{run_api_server, run_metrics_server, setup_tracing_subscriber};
//! use tokio::signal;
//!
//! #[tokio::main]
//! async fn main() {
//!     let args = parse_args().unwrap();
//!
//!     setup_tracing_subscriber(args.subscriber, args.verbose);
//!
//!     run_api_server(args.api_network.to_string(), args.api_port.to_string()).await;
//!
//!     if args.metrics {
//!         run_metrics_server(
//!             args.metrics_network.to_string(),
//!             args.metrics_port.to_string(),
//!         )
//!         .await;
//!     }
//!     signal::ctrl_c().await.unwrap();
//! }
//! ```
pub mod args;
pub mod workhours;

use axum::{
    extract::Query,
    http::{self, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};

use serde::Deserialize;
use tokio::task;
use tower_http::trace::TraceLayer;
use tracing::info;
use workhours::calculate_workhours;

/// Sets up the tracing subscriber based on the specified logging method and verbosity level.
///
/// This function configures the global tracing subscriber, which determines how
/// and where log messages are output. The configuration depends on the chosen
/// logging method and the verbosity level.
///
/// # Arguments
///
/// * `trace_method` - The method of logging to be used.
/// * `verbosity` - An integer representing the desired level of log verbosity.
///   Higher values result in more verbose logging.
///
/// # Supported Log Methods
///
/// * `LogMethod::Stdout` - Logs are written to standard output.
/// * `LogMethod::File` - (Not implemented) Intended for logging to a file.
/// * `LogMethod::Loki` - (Not implemented) Intended for logging to a Loki server.
///
/// # Verbosity Levels
///
/// The `verbosity` parameter is mapped to tracing levels as follows:
/// * 1 - ERROR
/// * 2 - WARN
/// * 3 - INFO
/// * 4 - DEBUG
/// * 5 and above - TRACE
///
/// # Example
///
/// ```
/// use time_tally::{setup_tracing_subscriber, args::LogMethod};
///
/// // Setup logging to stdout with INFO level verbosity
/// setup_tracing_subscriber(LogMethod::Stdout, 3);
/// ```
///
/// # Note
///
/// This function will panic if the subscriber fails to initialize.
pub fn setup_tracing_subscriber(trace_method: args::LogMethod, verbosity: u8) {
    match trace_method {
        args::LogMethod::File => {
            // TODO: Implement file logging
        }
        args::LogMethod::Stdout => {
            tracing_subscriber::fmt()
                .with_max_level(get_log_level(verbosity))
                .with_target(false)
                .compact()
                .init();
        }
        args::LogMethod::Loki => {
            // TODO: Implement Loki logging
        }
    }
}

// TODO: Investigate middleware stack as alternative:
/*
let middleware_stack = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(ConcurrencyLimitLayer::new(64))  // Limit concurrent requests
        .layer(TimeoutLayer::new(Duration::from_secs(30)))  // Set request timeout
        .into_inner();
*/

///Convert verbosity level
fn get_log_level(verbose: u8) -> tracing::Level {
    match verbose {
        1 => tracing::Level::ERROR,
        2 => tracing::Level::WARN,
        3 => tracing::Level::INFO,
        4 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    }
}

/// Runs the main API server.
///
/// Sets up routing, request tracing, and starts the server on the specified network and port.
///
/// # Arguments
///
/// * `network` - The network address to bind to.
/// * `port` - The port number to listen on.
///
/// # Panics
///
/// Panics if the server fails to bind to the specified address.
///
/// # Examples
///
/// ```no_run
/// use time_tally::run_api_server;
///
/// #[tokio::main]
/// async fn main() {
///     run_api_server("127.0.0.1".to_string(), "3000".to_string()).await;
/// }
/// ```
pub async fn run_api_server(network: String, port: String) {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(|request: &http::Request<_>, _span: &tracing::Span| {
            info!("started {} {}", request.method(), request.uri());
        })
        .on_response(
            |response: &http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                info!(
                    "response generated in {:?} with status {}",
                    latency,
                    response.status()
                );
            },
        );

    let router = Router::new()
        .route("/api/v1/workhours", get(get_workhours))
        .layer(trace_layer);
    // TODO: Other good layers to include?

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", network, port))
        .await
        .unwrap();

    tokio::spawn(async move {
        tracing::info!("API server listening on {}:{}", network, port);
        axum::serve(listener, router).await.unwrap();
    });
}

/// Runs the metrics server.
///
/// Sets up a separate server for serving metrics on the specified network and port.
///
/// # Arguments
///
/// * `network` - The network address to bind to.
/// * `port` - The port number to listen on.
///
/// # Panics
///
/// Panics if the server fails to bind to the specified address.
///
/// # Examples
///
/// ```no_run
/// use time_tally::run_metrics_server;
///
/// #[tokio::main]
/// async fn main() {
///     run_metrics_server("127.0.0.1".to_string(), "3001".to_string()).await;
/// }
/// ```
pub async fn run_metrics_server(network: String, port: String) {
    let router = Router::new().route("/metrics", get(get_metrics));

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", network, port))
        .await
        .unwrap();

    tokio::spawn(async move {
        tracing::info!("Metrics server listening on {}:{}", network, port);
        axum::serve(listener, router).await.unwrap();
    });
}

async fn get_metrics() -> &'static str {
    "hello world"
}

/// Handles requests to get work hours.
///
/// Calculates work hours based on the provided start and end dates.
///
/// # Arguments
///
/// * `Query(query)` - Query parameters containing start and end dates.
///
/// # Returns
///
/// Returns a JSON response with the calculated work hours or an error message.
async fn get_workhours(Query(query): Query<QueryParams>) -> impl IntoResponse {
    let result = task::spawn_blocking(move || calculate_workhours(query.start, query.end)).await;

    match result {
        Ok(Ok(workhours)) => Json(workhours).into_response(),
        Ok(Err(err)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Internal Server Error" })),
        )
            .into_response(),
    }
}

/// Represents the query parameters for the work hours calculation.
#[derive(Deserialize)]
struct QueryParams {
    /// The start date for the work hours calculation (format: "DD-MM-YYYY").
    start: String,
    /// The end date for the work hours calculation (format: "DD-MM-YYYY").
    end: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::Response;

    #[tokio::test]
    async fn test_get_workhours() {
        let query = Query(QueryParams {
            start: "01-01-2023".to_string(),
            end: "31-12-2023".to_string(),
        });

        let response: Response = get_workhours(query).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // You might want to add more assertions here to check the response body
    }

    // TODO: Add more tests as needed
}
