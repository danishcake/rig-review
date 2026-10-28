# Rig-review

A tool for performing LLM assisted reviews of code changes.

`rig-review` operates in terms of three components:

- A source of a diff.
- A sink for output.
- An engine that performs the review.

The following sources are available:

- Local git.<br>
  This reviews changes in a local git repository, using a pair of commit references.
- Gitlab.<br>
  This reviews changes against a gitlab merge request.

The following sinks are available:

- Stdout<br>
  This simples outputs the review to stdout
- Gitlab.<br>
  This posts comments against a gitlab merge request. It can only be used when the Gitlab source is used.
- Discord.<br>
  This posts to Discord using a Discord Webhook.

You can use multiple sinks by specifying the `--sink` option multiple times.

## Getting started

Before anything else you need to provide configuration.
Configuration is loaded from four sources of increasing priority:

- Environment variables.
- A local `rig-review.toml|rig-review.json` file.
- An explicitly set configuration file.
- Explicit CLI arguments. This only applies to a subset of configuration values.

Values in later configuration sources override earlier one.

### Rig review engine configuration

This section defines the LLM provider and which model is used.
This is the only mandatory section.

```toml
[rig-review-engine]
# The base URL for the OpenAI compatible endpoint
# This depends on your token provider
base_url = "http://127.0.0.1:1234/v1" # typical local lmstudio configuration
# For openrouter
# "base_url": "https://openrouter.ai/api/v1",
# Environment variable
# export RIG_REVIEW__rig_review_engine.base_url="http://127.0.0.1:1234/v1"

# OPTIONAL
# The API token for the OpenAI compatible endpoint
api_token = "sk-..."
# Environment variable
# export RIG_REVIEW__rig_review_engine.api_token="sk-..."

# The model to review with
model = "openai/gpt-oss-20b"
# ... and for openrouter you might use
# "model": "openai/gpt-6-luna",
# Environment variable
# export RIG_REVIEW__rig_review_engine.model="openai/gpt-oss-20b"

# OPTIONAL
# The maximum number of turns. Defaults to 30, which should be plenty
max_turns = 30
# Environment variable
# export RIG_REVIEW__rig_review_engine.max_turns=30
```

```json
{
  "rig-review-engine": {
    "base_url": "http://127.0.0.1:1234/v1",
    "api_token": "sk-...",
    "model": "openai/gpt-oss-20b",
    "max_turns": 30
  }
}
```

### Network configuration

Network configuration falls under the 'network' key and defines common network settings.

```toml
# OPTIONAL
[network]
# OPTIONAL
# Disables all TLS certificate verification. This can be dangerous, as it allows others to MitM
# your connection.
ignore_certs=true
# Environment variable
# export RIG_REVIEW__network.ignore_certs=true

# OPTIONAL
# Provides additional certificates.
extra_ca=["corp-domain.pem", "dev-domain.dev"]
# export RIG_REVIEW__network.extra_ca="corp-domain.pem,dev-domain.dev"
```

```json
{
  "network": {
    "ignore_certs": true,
    "extra_ca": ["corp-domain.pem", "dev-domain.dev"]
  }
}
```

### Gitlab source configuration

Gitlab configuration falls under the `gitlab` key and defines how the gitlab instance is accessed.

```toml
# OPTIONAL
[gitlab]
# OPTIONAL
# The host URL for the Gitlab instance. This is only required if you operate
# gitlab behind a reverse proxy, and use a custom path. For example, if you
# run gitlab on https://gitlab.my-org.lan then no configuration is required,
# but if you run it on https://my-org.lan/gitlab then you need to set that
# here.
host = "https://gitlab.com"
# Environment variable
# export RIG_REVIEW__gitlab.host="https://gitlab.com"

# An API token
# If using legacy coarse grained tokens then it the 'api' scope
# If using fine grained tokens then it needs the following scopes:
# Group and Projects/Projects/project.read
# Group and Projects/Projects/merge-request.read
# Group and Projects/Project planning/work-item.create
# User/System access/user.read
api_token = "glpat-..."
# export RIG_REVIEW__gitlab.api_token="glpat-..."
```

```json
{
  "gitlab": {
    "host": "https://gitlab.com",
    "api_token": "glpat-..."
  }
}
```

### HTTP server and GitLab webhook

The `rig-review-server` binary accepts GitLab merge request webhooks and queues a review when a
merge request is opened. It uses the same engine, GitLab, network, and environment configuration as
the CLI, with an additional `server` section:

```toml
[server]
# The IP to bind to. Defaults to 127.0.0.1
host = "0.0.0.0"
# Environment variable
# RIG_REVIEW__server.host="127.0.0.1"

# The port to listen on. Defaults to 8080
port = 8080
# RIG_REVIEW__server.port=8080

# Set this to the same secret configured in GitLab's webhook settings.
# Must not be empty
webhook_secret = "replace-with-a-long-random-secret"
# RIG_REVIEW__server.webhook_secret="replace-with-a-long-random-secret"

# Maximum number of waiting reviews, number of concurrent reviews, and retained job history.
queue_capacity = 100
# RIG_REVIEW__server.queue_capacity=100

worker_count = 1
# RIG_REVIEW__server.worker_count=1

job_history_limit = 1000
# RIG_REVIEW__server.job_history_limit=1000
```

The server requires `gitlab.api_token` and `gitlab.host`. In server mode, `gitlab.host` is used to
restrict webhook merge request URLs to the configured GitLab instance. The webhook secret is
separate from the GitLab API token.

In GitLab, add a project or group webhook with the URL `https://your-server/webhooks/gitlab`, set
the secret token to match `server.webhook_secret`, and enable **Merge request events**. Only the
open action queues a review. GitLab receives an accepted response with the queued job ID; other
event types and merge request actions are ignored.

A simple dashboard is served at `/` and refreshes automatically. The dashboard and read-only
endpoints used to populate it are public, so anyone who can reach the server can see merge request
URLs and job statuses.

Job state is held in memory and is lost when the server exits.

### Discord sink configuration

Discord configuration falls under the `discord` key and defines how Discord is accessed.

First, create a Webhook in Discord:

1. Right click the server and select `Server Settings/Integrations`.
2. Select `Webhooks`
3. Select `New Webhook`
4. Copy the URL. Note that it is formed of two parts - a number ID, and an alphanumeric token.

```
https://discord.com/api/webhooks/1234567891234567890/aslkdjsfjhlksdjfhkjsdhflkjsdhflksdahflkjadshflkjsdhflkjdsasdskfjsdlk
                                 ^ webhook_id        ^webhook_token
```

```toml
# OPTIONAL
[discord]
webhook_id = 1234
# Environment variable
# export RIG_REVIEW__discord.webhook_id="1234"

webhook_token = "ABCD"
# Environment variable
# export RIG_REVIEW__discord.webhook_token="ABCD"
```

```json
{
  "discord": {
    "webhook_id": 1234,
    "webhook_token": "ABCD"
  }
}
```
