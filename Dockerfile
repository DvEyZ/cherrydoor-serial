FROM rust:1.72
COPY . .
RUN cargo build --release
EXPOSE 9000
CMD [ "./target/release/cherrydoor-serial" ]
