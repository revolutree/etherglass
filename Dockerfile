FROM rust:1.61-buster as builder

# 2. Copy the files in your machine to the Docker image
COPY ./ ./
RUN apt-get update && apt-get install -y lsb-release && apt-get clean all
RUN apt-get install gcc binutils make curl -y
RUN curl -fsSL https://packages.redis.io/gpg | gpg --dearmor -o /usr/share/keyrings/redis-archive-keyring.gpg
RUN echo "deb [signed-by=/usr/share/keyrings/redis-archive-keyring.gpg] https://packages.redis.io/deb $(lsb_release -cs) main" | tee /etc/apt/sources.list.d/redis.list
RUN apt-get update
RUN apt install redis-server redis -y
# Build your program for release
RUN cargo build --release

# Run the binary
CMD ["./target/release/etherglass"]