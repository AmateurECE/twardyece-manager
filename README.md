# Generating a Certificate for Testing

```bash
# Generate a private key
openssl genrsa -aes256 -out test/manager.key

# Convert to PEM format
openssl rsa -in test/manager.key -text > test/manager-key.pem

# Generate a certificate signing request
openssl req -key test/manager.key -new -out test/manager.csr

# Sign the certificate
openssl x509 -signkey test/manager.key -in test/manager.csr -req -days 365 -out test/manager-cert.pem
```
