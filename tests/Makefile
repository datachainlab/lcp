OPENSSL   ?= openssl
CERTS_DIR ?= ./certs

.PHONY: gencerts
gencerts:
	$(OPENSSL) req -noenc -newkey rsa:3072 -pkeyopt rsa_keygen_pubexp:65537 -keyout $(CERTS_DIR)/root.key -out $(CERTS_DIR)/root.csr -subj "/C=US/ST=CA/L=Santa Clara/O=Example/CN=Test for Intel SGX Attestation Report Signing CA"
	$(OPENSSL) x509 -req -in $(CERTS_DIR)/root.csr -signkey $(CERTS_DIR)/root.key -days 10000 -out $(CERTS_DIR)/root.crt
	$(OPENSSL) verify -CAfile $(CERTS_DIR)/root.crt $(CERTS_DIR)/root.crt
	$(OPENSSL) req -noenc -newkey rsa:2048 -pkeyopt rsa_keygen_pubexp:65537 -keyout $(CERTS_DIR)/signing.key -out $(CERTS_DIR)/signing.csr -subj "/C=US/ST=CA/L=Santa Clara/O=Example/CN=Test for Intel SGX Attestation Report Signing"
	$(OPENSSL) x509 -req -in $(CERTS_DIR)/signing.csr -CA $(CERTS_DIR)/root.crt -CAkey $(CERTS_DIR)/root.key -CAcreateserial -days 10000 -out $(CERTS_DIR)/signing.crt
	$(OPENSSL) verify -CAfile $(CERTS_DIR)/root.crt $(CERTS_DIR)/signing.crt
	$(OPENSSL) x509 -in $(CERTS_DIR)/signing.crt -inform PEM -out $(CERTS_DIR)/signing.crt.der -outform DER
