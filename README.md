# GitHub Activity Check 🔍

A fast and reliable command-line tool written in Rust to check if a GitHub repository is actively maintained.

## Features

- **Quick Analysis**: Fetches key repository metrics in seconds
- **Smart Detection**: Uses multiple criteria to determine project health
- **Rate Limit Friendly**: Efficiently uses GitHub API with minimal requests
- **Token Support**: Works with or without GitHub authentication
- **Robust Parsing**: Handles edge cases and API limitations gracefully

## Installation

### From Source

```bash
git clone https://github.com/jozef-pridavok/github-activity-check.git
cd github-activity-check
cargo build --release
```

The binary will be available at `target/release/github-activity-check`

### Prerequisites

- Rust 1.70+ (uses 2024 edition)
- Internet connection for GitHub API access

## Usage

```bash
github-activity-check <owner> <repo>
```

### Examples

```bash
# Check if the Rust language repository is active
github-activity-check rust-lang rust

# Check a popular JavaScript framework
github-activity-check facebook react

# Check a smaller project
github-activity-check user-name project-name
```

### Sample Output

```
Repo: rust-lang/rust
-------------------------------------------
Commits total         : 156789
Contributors total    : 5432
Last commit           :
  sha                 : abc123def456...
  author              : John Doe <john@example.com>
  date (UTC)          : 2024-08-20 14:30:22 UTC
  message             : Fix memory leak in parser
-------------------------------------------
Project alive        : ALIVE ✅
Criteria: last ≤ 90 days or (contributors ≥ 3 and commits ≥ 100)
```

## Authentication

For higher rate limits and private repository access, set your GitHub token:

```bash
export GITHUB_TOKEN=your_github_personal_access_token
github-activity-check owner repo
```

### Creating a GitHub Token

1. Go to [GitHub Settings > Developer settings > Personal access tokens](https://github.com/settings/tokens)
2. Click "Generate new token (classic)"
3. Select appropriate scopes (public repositories don't need special permissions)
4. Copy the token and set it as an environment variable

## Activity Criteria

A repository is considered **ALIVE** if it meets any of these conditions:

- **Recent Activity**: Last commit within 90 days
- **Established Project**: Has 3+ contributors AND 100+ commits

This dual criteria approach helps identify both:
- Recently active projects (regardless of size)
- Mature projects that may have longer periods between commits

## Technical Details

### API Efficiency

The tool uses several optimization strategies:

1. **Link Header Parsing**: Extracts total counts from GitHub's pagination headers
2. **Search API Fallback**: Uses commit search when Link headers aren't available
3. **Minimal Requests**: Typically makes only 2-3 API calls per repository
4. **Smart Caching**: Leverages HTTP client connection pooling

### Dependencies

- **tokio**: Async runtime for efficient HTTP requests
- **reqwest**: HTTP client with TLS support
- **serde**: JSON serialization/deserialization
- **chrono**: Date/time handling with UTC support
- **anyhow**: Error handling and context

## Error Handling

The tool gracefully handles common scenarios:

- Repository not found (404)
- Rate limit exceeded (403)
- Network connectivity issues
- Invalid repository names
- Private repositories (without appropriate tokens)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development

```bash
# Run with debug output
cargo run -- owner repo

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

- [ ] Add JSON/CSV output formats
- [ ] Support for checking multiple repositories at once
- [ ] Additional activity metrics (issues, pull requests)
- [ ] Configuration file support
- [ ] Docker container support

## Why Rust?

This tool is written in Rust for several key advantages:

- **Performance**: Fast startup and execution times
- **Reliability**: Strong type system prevents runtime errors
- **Safety**: Memory safety without garbage collection overhead
- **Concurrency**: Excellent async support for API calls
- **Cross-platform**: Single binary works across operating systems

---

Made with ❤️ in Rust
