# GitHub Activity Check

A fast command-line tool to check if GitHub repositories are actively maintained.

## Quick Start

```bash
# Basic usage
github-activity-check rust-lang rust

# With GitHub token (higher rate limits)  
export GITHUB_TOKEN=your_token_here
github-activity-check facebook react
```

## Installation

### From Source

```bash
git clone https://github.com/jozef-pridavok/github-activity-check.git
cd github-activity-check
cargo build --release
```

Binary will be at `target/release/github-activity-check`

### Prerequisites

- Rust 1.70+
- Optional: GitHub token for higher rate limits

## Usage

### Basic Examples

```bash
# Check a repository
github-activity-check rust-lang rust

# Get JSON output
github-activity-check rust-lang rust --format json

# Extract specific values
github-activity-check rust-lang rust --format field:commits_total
# Output: 304969

github-activity-check rust-lang rust --format field:project_alive  
# Output: true

github-activity-check rust-lang rust --format field:last_release.tag_name
# Output: 1.89.0
```

### Available Fields

| Field | Description | Example |
|-------|-------------|---------|
| `commits_total` | Total number of commits | `304969` |
| `contributors_total` | Number of contributors | `7888` |
| `open_pull_requests` | Open PRs count | `789` |
| `open_issues` | Open issues count | `10645` |
| `project_alive` | Is project active? | `true` |
| `last_commit.sha` | Latest commit hash | `abc123...` |
| `last_commit.date_utc` | Latest commit date | `2025-09-06T00:11:48Z` |
| `last_commit.message` | Latest commit message | `Fix bug in parser` |
| `last_release.tag_name` | Latest release version | `1.89.0` |
| `last_release.name` | Latest release name | `Rust 1.89.0` |
| `last_release.date_utc` | Latest release date | `2025-08-07T10:55:11Z` |
| `last_release.is_prerelease` | Is prerelease version | `false` |

### Configuration File

Create `config.toml`:

```toml
format = "json"
min_commits = 100
min_contributors = 3
max_days = 60
max_release_days = 365
```

Use with:

```bash
github-activity-check rust-lang rust --config-file config.toml
```

### History Tracking

Track changes over time:

```bash
# First run - saves current state
github-activity-check rust-lang rust --history /tmp/rust.json

# Later runs - compares with saved state
github-activity-check rust-lang rust --history /tmp/rust.json --check commits_total
echo "Exit code: $?"
# Exit code: 0 = no change, 1 = small change, 2 = big change

# Use exit code in shell scripts
if github-activity-check rust-lang rust --history /tmp/rust.json --check project_alive; then
    echo "No change in project status"
else
    echo "Project status changed! (exit code: $?)"
fi
```

### Common Use Cases

#### Check if dependency is maintained
```bash
github-activity-check serde-rs serde --format field:project_alive
```

#### Monitor for new releases
```bash
github-activity-check rust-lang rust --history /tmp/rust.json --check last_release.tag_name
if [ $? -eq 1 ]; then
    echo "🎉 New Rust release detected!"
    # Send notification, update CI, etc.
fi
```

#### Monitor repository in script
```bash
#!/bin/bash
# Check if repository is alive
if github-activity-check user repo --format field:project_alive | grep -q "false"; then
    echo "WARNING: Repository appears inactive!"
fi

# Monitor for changes with exit codes
github-activity-check user repo --history /tmp/repo.json --check commits_total
if [ $? -ne 0 ]; then
    echo "Repository activity changed!"
    # Send notification, update dashboard, etc.
fi
```

#### Bulk analysis
```bash
# analyze-repos.sh - Single organization
for repo in "rust" "cargo" "rustup"; do
    echo -n "rust-lang/$repo: "
    if github-activity-check rust-lang $repo --format field:project_alive | grep -q "true"; then
        echo "✅ ACTIVE"
    else
        echo "❌ INACTIVE"
    fi
done

# Monitor multiple repos for changes (unique history files)
for repo in "rust" "cargo" "rustup"; do
    github-activity-check rust-lang $repo --history "/tmp/rust-lang-${repo}.json" --check project_alive
    if [ $? -eq 1 ]; then
        echo "⚠️  rust-lang/${repo}: Status changed"
    elif [ $? -eq 2 ]; then  
        echo "🚨 rust-lang/${repo}: Major change detected"
    fi
done

# Monitor repos from different owners
declare -A repos=(
    ["rust-lang"]="rust cargo"
    ["microsoft"]="vscode typescript" 
    ["facebook"]="react"
)

for owner in "${!repos[@]}"; do
    for repo in ${repos[$owner]}; do
        github-activity-check $owner $repo --history "/tmp/${owner}-${repo}.json" --check commits_total
        if [ $? -gt 0 ]; then
            echo "📈 ${owner}/${repo}: $? new commits"
        fi
    done
done
```

## How It Works

The tool analyzes repositories using multiple criteria:

- **Recent activity** (commits, issues, PRs)
- **Community size** (contributors)
- **Project maturity** (total commits)
- **Release activity** (recent releases, frequency)

A repository is considered "alive" if it has:
- Recent commits (within 60 days), OR  
- Established community (3+ contributors and 100+ commits), OR
- Recent releases (within 365 days) with active development

## Command Line Options

```
github-activity-check [OPTIONS] <OWNER> <REPO>

Options:
  --format <FORMAT>              Output format: default, json, field:name
  --config-file <FILE>           Load settings from TOML file
  --history <FILE>               Save/load run history
  --check <FIELD>                Check field changes (sets exit code)
  --min-commits <N>              Minimum commits threshold (default: 100)
  --min-contributors <N>         Minimum contributors threshold (default: 3)
  --max-days <N>                 Maximum days since last commit (default: 60)
  --max-release-days <N>         Maximum days since last release (default: 365)
  --verbose                      Show detailed output
  --help                         Show help
```

### Exit Codes (with --check)

Exit code represents **actual change magnitude**:

- **0** = No change detected
- **Positive number** = Magnitude of change (depends on field type)

**Change calculation by field type:**

| Field Type | Change Measurement | Exit Code = Actual Change |
|------------|-------------------|--------------------------|
| Numbers (`commits_total`) | Absolute difference | Exit code = |new - old| |
| Booleans (`project_alive`) | Status flip | 0 = same, 1 = different |
| Dates (`last_commit.date_utc`) | **Days difference** | **Exit code = days between commits** |
| Strings (`last_commit.sha`) | Text change | 0 = same, 1 = different |
| Release (`last_release.tag_name`) | Version change | 0 = same version, 1 = new release |

**Examples:**

```bash
# Numbers: Get absolute change
github-activity-check rust-lang rust --history /tmp/rust.json --check commits_total
echo "New commits since last check: $?"
# 0 = no new commits, 156 = 156 new commits

# Dates: Get days difference  
github-activity-check rust-lang rust --history /tmp/rust.json --check last_commit.date_utc
echo "Days since commit changed: $?"
# 0 = same commit, 3 = 3 days newer, 7 = 1 week newer

# Releases: Detect new version
github-activity-check rust-lang rust --history /tmp/rust.json --check last_release.tag_name
echo "Release change status: $?"
# 0 = same release, 1 = new release available

# Use in conditionals
if [ $? -gt 100 ]; then
    echo "More than 100 new commits!"
elif [ $? -gt 0 ]; then
    echo "Some activity detected ($? new commits)"
fi
```

Use in shell: `echo $?` or `if github-activity-check ...; then`

## Authentication

Set `GITHUB_TOKEN` environment variable to increase rate limits from 60 to 5000 requests/hour.

Get token at: https://github.com/settings/tokens (no permissions needed for public repos)

## License

MIT License - see LICENSE file for details.