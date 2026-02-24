SERVER="server"
TRUSTAUTH="trustauth"

# Check mode argument
MODE=${1:-}
if [ "$MODE" = "dev" ]; then
  SERVER_DNS="localhost"
  TRUSTAUTH_DNS="localhost"
elif [ "$MODE" = "prod" ]; then
  SERVER_DNS="rustsystem-server"
  TRUSTAUTH_DNS="rustsystem-trustauth"
else
  echo "Usage: $0 <dev|prod>"
  exit 1
fi

# Clear previous certs
rm -rf ca server trustauth
mkdir ca server trustauth

# CA key (Ed25519)
openssl genpkey -algorithm ED25519 -out ca/ca.key

# Self-signed CA cert
openssl req -x509 -new -key ca/ca.key -out ca/ca.crt -days 3650 \
  -subj "/C=SE/O=FSEK/OU=Infra/CN=Rustsystem Internal CA"

create_crts () {
  ENDPOINT=$1
  DNS_NAME=$2
  echo "
  [ req ]
    default_bits       = 2048
    prompt             = no
    default_md         = sha256
    distinguished_name = dn
    req_extensions     = req_ext

    [ dn ]
    C  = SE
    O  = MyOrg
    OU = Services
    CN = $ENDPOINT.internal

    [ req_ext ]
    subjectAltName = @alt_names
    extendedKeyUsage = serverAuth, clientAuth
    keyUsage = digitalSignature, keyEncipherment

    [ alt_names ]
    DNS.1 = $DNS_NAME
    $([ "$MODE" = "dev" ] && echo "IP.1  = 127.0.0.1")
  " > $ENDPOINT/openssl.cnf
  
  # # Key
  openssl genpkey -algorithm ED25519 -out $ENDPOINT/$ENDPOINT.key

  # CSR
  openssl req -new -key $ENDPOINT/$ENDPOINT.key -out $ENDPOINT/$ENDPOINT.csr \
    -config $ENDPOINT/openssl.cnf

  # Sign with CA
  openssl x509 -req \
    -in $ENDPOINT/$ENDPOINT.csr \
    -CA ca/ca.crt -CAkey ca/ca.key -CAcreateserial \
    -out $ENDPOINT/$ENDPOINT.crt \
    -days 825 \
    -extfile $ENDPOINT/openssl.cnf -extensions req_ext
}

create_crts $SERVER $SERVER_DNS
create_crts $TRUSTAUTH $TRUSTAUTH_DNS
