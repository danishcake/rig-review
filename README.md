# Rig-review

A tool for performing LLM assisted reviews of code changes.

`rig-review` operates in terms of three components:

- A source of a diff.
- A sink for output.
- An engine that performs the review.

The following soures are available:

- Local git.

## Configuration

Configuration is loaded from three sources of increasing priority:

- Environment variables.
- A local `rig-review.toml|rig-review.json` file.
- An explicitly set configuration file.

Values in later configuration sources override earlier one.

### Gitlab source configuration

Gitlab configuration falls under the `gitlab` key, and defines how the gitlab instance is accessed.

```toml
# OPTIONAL
[gitlab]
# OPTIONAL
# The host URL for the Gitlab instance. This is only required if you operate
# gitlab behind a reverse proxy, and use a custom path. For example, if you
# run gitlab on https://gitlab.my-org.lan then no configuration is required,
# but if you run it on https://my-org.lan/gitlab then you need to set that
# here.
host =  "https://gitlab.com"

# An API token
# If using legacy coarse grained tokens then it the 'api' scope
# If using fine grained tokens then it needs the following scopes:
# Group and Projects/Projects/project.read
# Group and Projects/Projects/merge-request.read
# Group and Projects/Project planning/work-item.create
# User/System access/user.read
api_token = "glpat-..."
```

```json
{
  "gitlab": {
    "host": "https://gitlab.com",
    "api_token": "glpat-..."
  }
}
```

### Rig review engine configuration

This section defines the LLM provider and which model is used.

```toml
[rig-review-engine]
# The base URL for the OpenAI compatible endpoint
# This depends on your token provider
base_url = "http://127.0.0.1:1234/v1" # typical local lmstudio configuration

# OPTIONAL
# The API token for the OpenAI compatible endpoint
api_token = "sk-..."

# The model to review with
model = "openai/gpt-oss-20b"

# OPTIONAL
# The maximum number of turns. Defaults to 30, which should be plenty
max_turns = 30
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
