# A pure-HTML front-end for StackMuncher

## Deployment

```
cargo build --release --target x86_64-unknown-linux-musl
cp ./target/x86_64-unknown-linux-musl/release/stm-html ./bootstrap && zip proxy.zip bootstrap && rm bootstrap
aws lambda update-function-code --region us-east-1 --function-name stm-html --zip-file fileb://proxy.zip
```