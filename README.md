# Work Hours Calculator API

This project provides an API server for calculating work hours between two dates, taking into account weekends and Swedish holidays.
It splits the workhours into periods which align with Unit4´s finance system Unit4 Business World (UBW). These periods are often aligned with the weeks,
but are adjusted so that a monthbreak never splits a period in two. Thus a perid can range from three to eleven days.

## Features

- Calculate work hours between two dates
- Exclude weekends and Swedish holidays from calculations
- HTTP API with JSON responses
- Optional metrics server
- Configurable logging methods
- Command-line argument parsing for easy configuration

## API Endpoints

- `GET /api/v1/workhours?start=DD-MM-YYYY&end=DD-MM-YYYY`
  - Calculate work hours between two dates
  - Returns a JSON response with work hours broken down by year, month, and week

- `GET /metrics` (if enabled)
  - Returns metrics data (currently a placeholder)

## Configuration

Use command-line arguments to configure the server:

- `-p, --api-port <PORT>`: Set the API server port (default: 3200)
- `-n, --api-network <IP>`: Set the API server network interface (default: 0.0.0.0)
- `-m, --metrics`: Enable metrics server
- `--metrics-port <PORT>`: Set the metrics server port (default: 3201)
- `--metrics-network <IP>`: Set the metrics server network interface (default: 0.0.0.0)
- `-s, --subscriber <METHOD>`: Set the logging method (options: file, loki, stdout; default: stdout)

## License

This project is dual-licensed:

- [Apache-2.0](https://spdx.org/licenses/Apache-2.0.html)
- [MIT](http://opensource.org/licenses/MIT)