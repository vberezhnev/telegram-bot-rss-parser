# Use the official Rust image as the base
FROM rust:1.71 as builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files to the container
COPY Cargo.toml Cargo.lock ./

# Create a new empty shell project to cache dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code to the container
COPY src ./src

# Build the actual application
RUN cargo build --release

# Use a smaller image for the final executable
FROM debian:buster-slim

# Set the working directory
WORKDIR /usr/local/bin

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/tech-chronicle-bot .

# Set the entry point for the container
ENTRYPOINT ["./tech-chronicle-bot"]
