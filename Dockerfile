# Build
FROM rust:slim AS build
ENV APP_NAME=online-judge

# Install build dependencies
RUN apt update && apt upgrade -y
RUN apt install -y sqlite3 && apt-get clean

WORKDIR /usr/src
RUN cargo new ${APP_NAME}
WORKDIR /usr/src/${APP_NAME}

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build and cache dependencies only
RUN cargo build --release
RUN rm src/*.rs

# Build binary
COPY build.rs judge.toml .
COPY judge.template.db ./judge.db
COPY migrations ./migrations
COPY src ./src
COPY static ./static
COPY templates ./templates
ENV DATABASE_URL=sqlite:///usr/src/${APP_NAME}/judge.db
RUN cargo build --release

# Run
FROM debian:stable-slim

ENV APP_NAME=online-judge
ENV BUILD_DIR=/usr/src/${APP_NAME}
ENV JUDGE_DIR=/judge

ENV SERVER_ADDRESS=0.0.0.0:80
ENV DATABASE_URL=sqlite://${JUDGE_DIR}/judge.db
ENV CONTEST_DIR=${JUDGE_DIR}/contests
ENV STATIC_DIR=${JUDGE_DIR}/static
ENV JUDGE_CONFIG=${JUDGE_DIR}/judge.toml

# Install runtime dependencies
RUN apt update && apt upgrade -y
RUN apt install -y sqlite3 gcc g++ python3 && apt-get clean

WORKDIR ${JUDGE_DIR}

COPY --from=build ${BUILD_DIR}/target/release/${APP_NAME} /usr/local/bin/${APP_NAME}
COPY --from=build ${BUILD_DIR}/judge.db ${JUDGE_DIR}/judge.db
COPY --from=build ${BUILD_DIR}/static ${STATIC_DIR}
COPY --from=build ${BUILD_DIR}/judge.toml ${JUDGE_DIR}/judge.toml
RUN mkdir /judge/contests

VOLUME ${JUDGE_DIR}
EXPOSE 80
ENV RUST_LOG=online_judge=trace,tower_http=trace

CMD ${APP_NAME}
