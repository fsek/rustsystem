SERVER="server"
TRUSTAUTH="trustauth"

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
    DNS.1 = $ENDPOINT.internal
    DNS.2 = localhost
    IP.1  = 127.0.0.1
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

create_crts $SERVER
create_crts $TRUSTAUTH
