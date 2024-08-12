# @database-playground/dbrunner

The SQLite-backed arbitrary SQL query executor, written in Rust for the optimized resource usage.

It provides a simple gRPC interface to execute arbitrary SQL queries with a schema, retrieve the results in a structured format, and compare the results with the expected ones.

The query is cached in an efficient format to avoid rerunning the query each time. This also makes the quick comparison with the expected results possible.

## Usage

You should prepare your Redis database. After that, set the `REDIS_URI` environment variable to the URI of your Redis database.

```bash
export REDIS_URI=redis://localhost:6379
```

Then, build and run the server:

```bash
cargo run --release
```

You can also build dbrunner with Nix:

```bash
nix build .
```

## Deployment

Deploy it directly to [Zeabur](https://zeabur.com) with the following command:

```bash
npx zeabur deploy
```

You can also leverage the Nix package manager to build the Docker image for deployment:

```bash
# You should prepare a Linux remote builder.
nix build .#packages.aarch64-linux.docker  # For ARM64
nix build .#packages.x86_64-linux.docker   # For x86_64
docker load < result
```

## Development

We use [Devenv](https://devenv.sh) to manage the development environment, and [VS Code](https://code.visualstudio.com) as the IDE of Rust.

Install the recommended extensions and run `direnv allow && direnv reload` to set up the development environment.

Run `devenv up` to start the Redis service.

For interacting with the gRPC API, `grpcui` is a good choice:

```bash
[nix-shell] $ grpcui -plaintext -proto ./proto/dbrunner.proto 127.0.0.1:50051
```

## License

This project is licensed under the AGPL-3.0-or-later license.
