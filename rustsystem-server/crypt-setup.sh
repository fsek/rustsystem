# Prepare out directory
mkdir -p certs
cd certs

# Certificates expire after one year
TIMEOUT=365

# Generate root CA
openssl genrsa -out rootCA.key 4096
openssl req -x509 -new -nodes -key rootCA.key -sha256 -days $TIMEOUT -out rootCA.pem -subj "/CN=RootCA"


# Generate client CA
# This is the intermediate CA used for signing clients

# Temporary file for client config
touch clientCA.cnf
echo "basicConstraints=CA:TRUE,pathlen:0" >> clientCA.cnf

openssl genrsa -out clientCA.key 4096
openssl req -new -key clientCA.key -out clientCA.csr -subj "/CN=ClientCA"
openssl x509 -req -in clientCA.csr -CA rootCA.pem -CAkey rootCA.key \
  -CAcreateserial -out clientCA.pem -days 1825 -sha256 \
  -extfile clientCA.cnf

# Remove temporary config file
rm clientCA.cnf

# Generate server key and CSR
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr -config ../config/server.cnf
# Sign server CSR with root CA
openssl x509 -req -in server.csr -CA rootCA.pem -CAkey rootCA.key -CAcreateserial \
  -out server.crt -days $TIMEOUT -sha256 -extfile ../config/server.cnf -extensions req_ext
