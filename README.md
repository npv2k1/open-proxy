# Open Proxy

A multi-threaded proxy parser and checker with support for various proxy formats. Parse, validate, and save good/bad proxies efficiently.

## Features

- 🔄 **Multi-format Proxy Parsing**: Parse proxies from various formats (IP:PORT, IP:PORT:USER:PASS, URL format, etc.)
- ⚡ **Multi-threaded Checking**: Concurrent proxy validation with configurable thread count
- 📁 **Separate Output**: Save good and bad proxies to different files
- 🔧 **Flexible Configuration**: Customizable timeout, test URL, and proxy types
- 📝 **Multiple Proxy Types**: Support for HTTP, HTTPS, SOCKS4, and SOCKS5 proxies
- 🌐 **Web Crawler**: Crawl and extract proxies from websites and proxy lists
- 🖥️ **Interactive Terminal User Interface (TUI)**
- 🧪 **Comprehensive test suite**
- 🚀 **CI/CD with GitHub Actions**

## Installation

### From Source

```bash
git clone https://github.com/npv2k1/open-proxy.git
cd open-proxy
cargo build --release
```

### From Releases

Download the latest binary from the [Releases](https://github.com/npv2k1/open-proxy/releases) page.

## Usage

### Proxy Parser

Parse proxies from a file and output them in a standardized format:

```bash
# Parse proxies from a file and print to stdout
./open-proxy parse proxies.txt

# Parse and save to output file
./open-proxy parse proxies.txt -o parsed_proxies.txt

# Specify proxy type (http, https, socks4, socks5)
./open-proxy parse proxies.txt -t socks5 -o socks_proxies.txt
```

### Proxy Checker

Check proxies and separate working from non-working ones:

```bash
# Check proxies and save results
./open-proxy check proxies.txt --good good.txt --bad bad.txt

# Check with custom settings
./open-proxy check proxies.txt \
  --good good.txt \
  --bad bad.txt \
  --threads 20 \
  --timeout 15 \
  --test-url "http://httpbin.org/ip"

# Check SOCKS5 proxies
./open-proxy check proxies.txt -t socks5 --good working_socks.txt
```

### Proxy Crawler

Crawl and extract proxies from websites:

```bash
# Crawl proxies from a specific URL
./open-proxy crawl --url https://example.com/proxy-list.txt -o proxies.txt

# Crawl from multiple URLs
./open-proxy crawl \
  --url https://example.com/list1.txt \
  --url https://example.com/list2.txt \
  -o all_proxies.txt

# Use built-in common proxy sources
./open-proxy crawl --common-sources -o proxies.txt

# Crawl with custom timeout and proxy type
./open-proxy crawl \
  --url https://example.com/socks.txt \
  -t socks5 \
  --timeout 60 \
  -o socks_proxies.txt
```

### Supported Proxy Formats

The parser supports multiple proxy formats:

- `IP:PORT` - Simple format (e.g., `192.168.1.1:8080`)
- `IP:PORT:USER:PASS` - With authentication (e.g., `192.168.1.1:8080:user:pass`)
- `USER:PASS@IP:PORT` - Alternative auth format (e.g., `user:pass@192.168.1.1:8080`)
- `scheme://IP:PORT` - URL format (e.g., `http://192.168.1.1:8080`)
- `scheme://USER:PASS@IP:PORT` - URL with auth (e.g., `socks5://user:pass@192.168.1.1:1080`)

### Example Input File

```
# HTTP proxies
192.168.1.1:8080
192.168.1.2:8080:user:pass
http://192.168.1.3:8080

# SOCKS5 proxies
socks5://192.168.1.4:1080
socks5://user:pass@192.168.1.5:1080
```

### Command Options

#### Parse Command

```
Usage: open-proxy parse [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input file containing proxies

Options:
  -o, --output <OUTPUT>          Output file for parsed proxies
  -t, --proxy-type <PROXY_TYPE>  Proxy type (http, https, socks4, socks5) [default: http]
  -h, --help                     Print help
```

#### Check Command

```
Usage: open-proxy check [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input file containing proxies

Options:
  -g, --good <GOOD>              Output file for good proxies
  -b, --bad <BAD>                Output file for bad proxies
  -t, --proxy-type <PROXY_TYPE>  Proxy type (http, https, socks4, socks5) [default: http]
  -n, --threads <THREADS>        Number of concurrent threads [default: 10]
      --timeout <TIMEOUT>        Timeout in seconds [default: 10]
      --test-url <TEST_URL>      URL to test proxies against [default: http://httpbin.org/ip]
  -h, --help                     Print help
```

#### Crawl Command

```
Usage: open-proxy crawl [OPTIONS]

Options:
  -u, --url <URL>                URLs to crawl proxies from (can specify multiple)
  -o, --output <OUTPUT>          Output file for crawled proxies
  -t, --proxy-type <PROXY_TYPE>  Proxy type (http, https, socks4, socks5) [default: http]
      --timeout <TIMEOUT>        Timeout in seconds for HTTP requests [default: 30]
      --common-sources           Use common free proxy sources
  -h, --help                     Print help
```

## Project Structure

```
open-proxy/
├── src/
│   ├── proxy/            # Proxy module
│   │   ├── mod.rs        # Module exports
│   │   ├── models.rs     # Proxy data models
│   │   ├── parser.rs     # Proxy parser
│   │   ├── checker.rs    # Multi-threaded proxy checker
│   │   └── crawler.rs    # Web crawler for proxy extraction
│   ├── database/         # Database layer
│   ├── models/           # Data models
│   ├── tui/              # Terminal UI
│   ├── lib.rs            # Library root
│   └── main.rs           # CLI application
├── tests/                # Integration tests
└── examples/             # Usage examples
```

## Development

### Prerequisites

- Rust 1.70 or later

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Clippy (Linter)

```bash
cargo clippy -- -D warnings
```

### Formatting Code

```bash
cargo fmt
```

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
